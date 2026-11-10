#!/usr/bin/env node
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Command } from 'commander';
import { execaSync } from 'execa';
import { workerHost, workerSsh, sshOptions } from '../worker-ssh.mts';
import { RUNNER_PATH } from '../prepare-build.mts';
import { createSnapshot } from '../snapshot.mts';
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = new Command().option('--source <checkout>', 'source checkout', process.cwd()).option('--host <host>', 'SSH worker', 'doramcap-ci@100.64.0.20').parse().opts();
  const temporary = mkdtempSync(join(tmpdir(), 'doramcap-submit-')), job = '/home/doramcap-ci/jobs/' + randomUUID().replaceAll('-', '');
  try {
    const host = workerHost(args.host);
    const expectedNode = 'v' + readFileSync(join(args.source, '.node-version'), 'utf8').trim();
    assert.equal(workerSsh(host, '"$HOME/.local/bin/node" --version').trim(), expectedNode, `Worker Node must match ${expectedNode}; run install-node.mts --host ${host}`);
    const archive = join(temporary, 'source.tar.gz'), { checksum } = await createSnapshot(args.source, archive);
    workerSsh(host, `mkdir -p ${job}`, true);
    execaSync('scp', [...sshOptions, archive, `${host}:${job}/source.tar.gz`], { stdio: 'inherit' });
    workerSsh(host, `cd ${job} && echo "${checksum}  source.tar.gz" | sha256sum -c - && tar -xzpf source.tar.gz && pueue add --label doramcap-native -- "$HOME/.local/bin/node" ${RUNNER_PATH}`, true);
    console.log(`Worker snapshot: ${host}:${job}`);
  } catch (error) { console.error((error as Error).message); process.exitCode = 1; }
  finally { rmSync(temporary, { recursive: true, force: true }); }
}
