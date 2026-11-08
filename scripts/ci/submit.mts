#!/usr/bin/env node
// Snapshot a source checkout and enqueue ARM64 tests over SSH.
import { randomUUID } from 'node:crypto';
import { lstatSync, readFileSync, realpathSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { writeTarGzip, type ArchiveEntry } from '../../libs/tooling/archive.mts';
import { Command, cli, isMain } from '../../libs/tooling/cli.mts';
import { digest, sha256, withTempDirectory } from '../../libs/tooling/fs.mts';
import { globMatcher } from '../../libs/tooling/glob.mts';
import { capture, run } from '../../libs/tooling/process.mts';
import { PREPARE_PATH, type SnapshotManifest } from './prepare-build.mts';

const ciDirectory = dirname(fileURLToPath(import.meta.url));
const sshOptions = ['-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10',
  '-o', 'ServerAliveInterval=10', '-o', 'ServerAliveCountMax=3'];
const included = globMatcher(['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.node-version',
  'libs/**', 'apps/**', 'console/**', 'scripts/ci/check-boundaries.mts', 'scripts/ci/test-boundaries.mts',
  'scripts/ci/boundaries.json', 'scripts/ci/test-node.mts', 'scripts/ci/test-mcap.mts', PREPARE_PATH,
  'scripts/ci/test-prepare-build.mts', 'scripts/ci/submit.mts', 'scripts/ci/test-submit.mts', 'scripts/ci/run-worker-job.sh']);

/** Create an allowlisted tar.gz and its SOURCE.json manifest from tracked and non-ignored source files. */
export async function createSnapshot(source: string, archive: string): Promise<SnapshotManifest> {
  source = realpathSync(source);
  for (const required of ['apps/system-monitor/tests/validate_native.ts', 'libs/resources/cgroup-bpf/examples/native_allocations.rs', PREPARE_PATH]) {
    if (!statSync(join(source, required), { throwIfNoEntry: false })?.isFile()) {
      throw new Error(`Source lacks ${required}; use a checkout containing the native validation suite and CI tools`);
    }
  }
  const git = (...args: string[]) => capture('git', ['-C', source, ...args]).stdout;
  const paths = [...new Set(git('ls-files', '-z', '--cached', '--others', '--exclude-standard').split('\0'))].filter(included).sort();
  const manifest: SnapshotManifest = {
    head: git('rev-parse', 'HEAD').trim(), dirty: Boolean(git('status', '--porcelain')),
    files_sha256: {}, files_mode: {}, runner_sha256: '', prepare_sha256: '',
  };
  function* entries(): Generator<ArchiveEntry> {
    for (const name of paths) {
      const path = join(source, name);
      const status = lstatSync(path, { throwIfNoEntry: false });
      if (!status) continue;
      if (!status.isFile()) throw new Error(`Refusing non-regular source file: ${name}`);
      const data = readFileSync(path);
      manifest.files_sha256[name] = sha256(data);
      manifest.files_mode[name] = status.mode & 0o777;
      yield { name, data, mode: status.mode & 0o777 };
    }
    const runner = readFileSync(join(ciDirectory, 'run-worker-job.sh'));
    manifest.runner_sha256 = sha256(runner);
    manifest.prepare_sha256 = manifest.files_sha256[PREPARE_PATH];
    yield { name: 'run-worker-job.sh', data: runner, mode: 0o644 };
    yield { name: 'SOURCE.json', data: Buffer.from(JSON.stringify(manifest, null, 2)), mode: 0o644 };
  }
  await writeTarGzip(archive, entries());
  return manifest;
}

if (isMain(import.meta.url)) await cli(new Command()
  .description('Snapshot a source checkout and enqueue ARM64 tests over SSH.')
  .option('--source <checkout>', 'source checkout', process.cwd())
  .option('--host <host>', 'SSH worker', 'doramcap-ci@100.64.0.20')
  .action(async (values: { source: string; host: string }) => withTempDirectory('doramcap-submit-', async temporary => {
    const job = '/home/doramcap-ci/jobs/' + randomUUID().replaceAll('-', '');
    const archive = join(temporary, 'source.tar.gz');
    await createSnapshot(values.source, archive);
    const checksum = digest(archive);
    run('ssh', [...sshOptions, values.host, `mkdir -p ${job}`]);
    run('scp', [...sshOptions, archive, `${values.host}:${job}/source.tar.gz`]);
    run('ssh', [...sshOptions, values.host, `cd ${job} && echo "${checksum}  source.tar.gz" | sha256sum -c - && `
      + 'tar -xzpf source.tar.gz && pueue add --label doramcap-native -- bash run-worker-job.sh']);
    console.log(`Worker snapshot: ${values.host}:${job}`);
  })));
