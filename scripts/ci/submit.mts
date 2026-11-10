#!/usr/bin/env node
import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { chmodSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Command } from 'commander';
import { execaSync } from 'execa';
import { minimatch } from 'minimatch';
import { PREPARE_PATH, type SnapshotManifest } from './prepare-build.mts';
const included = ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.node-version', 'libs/**', 'apps/**', 'console/**',
  'scripts/ci/{check-boundaries,test-boundaries,test-node,test-mcap,prepare-build,test-prepare-build,submit,test-submit,submit-run-only,test-run-only}.mts',
  'scripts/ci/{boundaries.json,package.json,package-lock.json,tsconfig.json,run-worker-job.sh,run-only-worker.sh,install-nextest.sh,run-only-artifacts.txt}'];
const hash = (data: Uint8Array) => createHash('sha256').update(data).digest('hex');
/** Stage exact source bytes and permissions; tar gives every entry one timestamp. Always remove staging files. */
export async function createSnapshot(source: string, archive: string): Promise<{ manifest: SnapshotManifest; checksum: string }> {
  source = realpathSync(source);
  for (const name of ['apps/system-monitor/tests/validate_native.ts', 'libs/resources/cgroup-bpf/examples/native_allocations.rs', PREPARE_PATH]) {
    assert(statSync(join(source, name), { throwIfNoEntry: false })?.isFile(), `Source lacks ${name}; use a checkout containing the native validation suite and CI tools`);
  }
  const git = (args: string[]) => execaSync('git', args, { cwd: source, stripFinalNewline: false, maxBuffer: Infinity }).stdout;
  const manifest: SnapshotManifest = { head: git(['rev-parse', 'HEAD']).trim(), dirty: Boolean(git(['status', '--porcelain'])),
    files_sha256: {}, files_mode: {}, runner_sha256: '', prepare_sha256: '' };
  const staged = mkdtempSync(join(tmpdir(), 'doramcap-source-'));
  try {
    for (const name of [...new Set(git(['ls-files', '-z', '--cached', '--others', '--exclude-standard']).split('\0'))].sort()) {
      if (!included.some(pattern => minimatch(name, pattern, { dot: true }))) continue;
      const status = lstatSync(join(source, name), { throwIfNoEntry: false });
      if (!status) continue;
      assert(status.isFile(), `Refusing non-regular source file: ${name}`);
      const data = readFileSync(join(source, name)), target = join(staged, name);
      mkdirSync(dirname(target), { recursive: true });
      writeFileSync(target, data);
      chmodSync(target, manifest.files_mode[name] = status.mode & 0o777);
      manifest.files_sha256[name] = hash(data);
    }
    const runner = readFileSync(new URL('./run-worker-job.sh', import.meta.url));
    manifest.runner_sha256 = hash(runner);
    manifest.prepare_sha256 = manifest.files_sha256[PREPARE_PATH];
    writeFileSync(join(staged, 'run-worker-job.sh'), runner);
    writeFileSync(join(staged, 'SOURCE.json'), JSON.stringify(manifest));
    for (const name of ['SOURCE.json', 'run-worker-job.sh']) chmodSync(join(staged, name), 0o644);
    execaSync('tar', ['-czf', resolve(archive), `--mtime=@${Math.floor(Date.now() / 1000)}`, '-C', staged, '.']);
    return { manifest, checksum: hash(readFileSync(archive)) };
  } finally { rmSync(staged, { recursive: true, force: true }); }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = new Command().option('--source <checkout>', 'source checkout', process.cwd()).option('--host <host>', 'SSH worker', 'doramcap-ci@100.64.0.20').parse().opts();
  const temporary = mkdtempSync(join(tmpdir(), 'doramcap-submit-')), job = '/home/doramcap-ci/jobs/' + randomUUID().replaceAll('-', '');
  const ssh = ['-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10', '-o', 'ServerAliveInterval=10', '-o', 'ServerAliveCountMax=3'];
  try {
    const archive = join(temporary, 'source.tar.gz'), { checksum } = await createSnapshot(args.source, archive);
    execaSync('ssh', [...ssh, args.host, `mkdir -p ${job}`], { stdio: 'inherit' });
    execaSync('scp', [...ssh, archive, `${args.host}:${job}/source.tar.gz`], { stdio: 'inherit' });
    execaSync('ssh', [...ssh, args.host, `cd ${job} && echo "${checksum}  source.tar.gz" | sha256sum -c - && tar -xzpf source.tar.gz && pueue add --label doramcap-native -- bash run-worker-job.sh`], { stdio: 'inherit' });
    console.log(`Worker snapshot: ${args.host}:${job}`);
  } catch (error) { console.error((error as Error).message); process.exitCode = 1; }
  finally { rmSync(temporary, { recursive: true, force: true }); }
}
