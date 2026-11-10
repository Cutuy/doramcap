#!/usr/bin/env node
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Command } from 'commander';
import { execaSync } from 'execa';
import { createSnapshot } from '../snapshot.mts';
import { verifyArtifactInventory } from './artifacts.mts';
import { sshOptions, workerHost, workerSsh as ssh } from '../worker-ssh.mts';

/** Require the exact queued build to succeed before downloading any artifacts. */
export function successfulBuild(status: string, id: string): void {
  assert(/^\d+$/.test(id), 'Invalid Pueue task ID');
  const task = JSON.parse(status).tasks?.[id];
  assert.equal(task?.status?.Done?.result, 'Success', `Pueue build ${id} did not succeed: ${JSON.stringify(task?.status)}`);
}
/** Submit a source snapshot, await its queued ARM64 build, then execute the archives remotely. */
export async function submitRunOnly(source: string, builder: string, runner: string): Promise<void> {
  builder = workerHost(builder); runner = workerHost(runner);
  const temporary = mkdtempSync(join(tmpdir(), 'doramcap-run-only-'));
  const job = 'jobs/' + randomUUID().replaceAll('-', '');
  try {
    const expectedNode = 'v' + readFileSync(join(source, '.node-version'), 'utf8').trim();
    for (const host of new Set([builder, runner])) {
      const actual = ssh(host, 'if test -x "$HOME/.local/bin/node"; then "$HOME/.local/bin/node" --version; fi').trim();
      assert.equal(actual, expectedNode, `${host}: Node ${actual || 'missing'}; expected ${expectedNode}. Run node scripts/ci/install-node.mts --host ${host}`);
    }
    const archive = join(temporary, 'source.tar.gz');
    const { checksum } = await createSnapshot(source, archive);
    ssh(builder, `mkdir -p ${job}/source`);
    execaSync('scp', [...sshOptions, archive, `${builder}:${job}/source/source.tar.gz`], { stdio: 'inherit' });
    const id = ssh(builder, `cd ${job}/source && echo '${checksum}  source.tar.gz' | sha256sum -c - >&2 && tar -xzpf source.tar.gz && cd .. && pueue add --print-task-id --label doramcap-run-only-build -- "$HOME/.local/bin/node" source/scripts/ci/macos-linux/worker.mts build`).trim();
    assert(/^\d+$/.test(id), `Invalid Pueue task ID: ${id}`);
    console.log(`Builder: ${builder}:${job} (Pueue ${id})`);
    console.log(`Runner: ${runner}:${job}`);
    ssh(builder, `pueue wait --quiet ${id}`, true);
    try { successfulBuild(ssh(builder, 'pueue status --json'), id); }
    catch (error) { ssh(builder, `pueue log --full ${id}`, true); throw error; }
    verifyArtifactInventory(ssh(builder, `cat ${job}/artifacts/SHA256SUMS`));
    ssh(runner, `mkdir -p ${job}/artifacts ${job}/source`);
    execaSync('scp', [...sshOptions, '-3', '-r', `${builder}:${job}/artifacts/.`, `${runner}:${job}/artifacts/`], { stdio: 'inherit' });
    verifyArtifactInventory(ssh(runner, `cat ${job}/artifacts/SHA256SUMS`));
    ssh(runner, `cd ${job}/artifacts && sha256sum -c SHA256SUMS && tar -xzpf source.tar.gz -C ../source && cd .. && { "$HOME/.local/bin/node" source/scripts/ci/macos-linux/worker.mts run; worker_result=$?; test ! -f results.tsv || cat results.tsv; echo "Runner log: $PWD/run.log"; exit "$worker_result"; }`, true);
  } finally { rmSync(temporary, { recursive: true, force: true }); }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = new Command().option('--source <checkout>', 'source checkout', process.cwd())
    .option('--build-host <host>', 'native ARM64 build worker', 'doramcap-ci@100.64.0.20')
    .option('--run-host <host>', 'run-only ARM64 worker', 'mac-mini-ubuntu24@100.64.0.22').parse().opts();
  try { await submitRunOnly(args.source, args.buildHost, args.runHost); }
  catch (error) { console.error((error as Error).message); process.exitCode = 1; }
}
