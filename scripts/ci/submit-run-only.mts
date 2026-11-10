#!/usr/bin/env node
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Command } from 'commander';
import { execaSync } from 'execa';
import { createSnapshot } from './submit.mts';

const sshOptions = ['-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10', '-o', 'ServerAliveInterval=10', '-o', 'ServerAliveCountMax=3'];
export const artifactNames = readFileSync(new URL('./run-only-artifacts.txt', import.meta.url), 'utf8').trimEnd().split('\n');
/** Require the complete artifact inventory before remote verification or extraction. */
export function verifyArtifactInventory(manifest: string): void {
  const remaining = new Set(artifactNames), lines = manifest.split('\n');
  if (lines.at(-1) === '') lines.pop();
  for (const line of lines) {
    const match = /^[a-f0-9]{64}  ([a-z][a-z0-9.-]*)$/i.exec(line);
    assert(match, `Malformed checksum entry: ${line}`);
    assert(remaining.delete(match[1]), `Unexpected or duplicate artifact: ${match[1]}`);
  }
  assert.equal(remaining.size, 0, `Missing checksum entries: ${[...remaining].join(', ')}`);
}
/** Accept SSH account/host names; reject option injection and remote-shell syntax. */
export function workerHost(value: string): string {
  assert(/^(?:[a-zA-Z0-9_][a-zA-Z0-9_.-]*@)?[a-zA-Z0-9][a-zA-Z0-9.-]*$/.test(value), `Invalid SSH worker: ${value}`);
  return value;
}
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
  const ssh = (host: string, command: string, inherit = false) => execaSync('ssh', [...sshOptions, host, command],
    inherit ? { stdio: 'inherit' } : { maxBuffer: Infinity }).stdout ?? '';
  try {
    const archive = join(temporary, 'source.tar.gz');
    const { checksum } = await createSnapshot(source, archive);
    ssh(builder, `mkdir -p ${job}/source`);
    execaSync('scp', [...sshOptions, archive, `${builder}:${job}/source/source.tar.gz`], { stdio: 'inherit' });
    const id = ssh(builder, `cd ${job}/source && echo '${checksum}  source.tar.gz' | sha256sum -c - >&2 && tar -xzpf source.tar.gz && cd .. && pueue add --print-task-id --label doramcap-run-only-build -- bash source/scripts/ci/run-only-worker.sh build`).trim();
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
    ssh(runner, `cd ${job}/artifacts && sha256sum -c SHA256SUMS && tar -xzpf source.tar.gz -C ../source && cd .. && { bash source/scripts/ci/run-only-worker.sh run; worker_result=$?; test ! -f results.tsv || cat results.tsv; echo "Runner log: $PWD/run.log"; exit "$worker_result"; }`, true);
  } finally { rmSync(temporary, { recursive: true, force: true }); }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = new Command().option('--source <checkout>', 'source checkout', process.cwd())
    .option('--build-host <host>', 'native ARM64 build worker', 'doramcap-ci@100.64.0.20')
    .option('--run-host <host>', 'run-only ARM64 worker', 'mac-mini-ubuntu24@100.64.0.22').parse().opts();
  try { await submitRunOnly(args.source, args.buildHost, args.runHost); }
  catch (error) { console.error((error as Error).message); process.exitCode = 1; }
}
