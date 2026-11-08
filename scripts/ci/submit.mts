#!/usr/bin/env node
// Snapshot a source checkout and enqueue ARM64 tests over SSH.
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { chmodSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';
import { digest, type SnapshotManifest } from './prepare-build.mts';

const ciDirectory = dirname(fileURLToPath(import.meta.url));
const sshOptions = ['-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10',
  '-o', 'ServerAliveInterval=10', '-o', 'ServerAliveCountMax=3'];
const sourceFiles = new Set(['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.node-version',
  'scripts/ci/check-boundaries.mts', 'scripts/ci/test-boundaries.mts', 'scripts/ci/boundaries.json',
  'scripts/ci/test-node.mts', 'scripts/ci/test-mcap.mts', 'scripts/ci/tsconfig.json',
  'scripts/ci/prepare-build.mts', 'scripts/ci/test-prepare-build.mts',
  'scripts/ci/submit.mts', 'scripts/ci/test-submit.mts', 'scripts/ci/run-worker-job.sh']);

/** Create an allowlisted tar.gz and its SOURCE.json manifest from tracked and non-ignored source files. */
export function createSnapshot(source: string, archive: string): SnapshotManifest {
  source = realpathSync(source);
  for (const required of ['apps/system-monitor/tests/validate_native.ts', 'libs/resources/cgroup-bpf/examples/native_allocations.rs']) {
    if (!statSync(join(source, required), { throwIfNoEntry: false })?.isFile()) {
      throw new Error(`Source lacks ${required}; use a checkout containing the native validation suite`);
    }
  }
  const git = (...args: string[]) => execFileSync('git', ['-C', source, ...args], { encoding: 'utf8', maxBuffer: Infinity });
  const paths = [...new Set(git('ls-files', '-z', '--cached', '--others', '--exclude-standard').split('\0'))]
    .filter(path => sourceFiles.has(path) || ['libs/', 'apps/', 'console/'].some(prefix => path.startsWith(prefix))).sort();
  const manifest: SnapshotManifest = {
    head: git('rev-parse', 'HEAD').trim(), dirty: Boolean(git('status', '--porcelain')),
    files_sha256: {}, files_mode: {}, runner_sha256: '', prepare_sha256: '',
  };
  const stage = mkdtempSync(join(tmpdir(), 'doramcap-snapshot-'));
  const names: string[] = [];
  const snapshotTime = Date.now() / 1000;
  const add = (name: string, data: Buffer, mode: number) => {
    const path = join(stage, name);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, data);
    chmodSync(path, mode);
    names.push(name);
  };
  try {
    for (const relative of paths) {
      const path = join(source, relative);
      const status = lstatSync(path, { throwIfNoEntry: false });
      if (!status) continue;
      if (!status.isFile()) throw new Error(`Refusing non-regular source file: ${relative}`);
      add(relative, readFileSync(path), status.mode & 0o777);
      manifest.files_sha256[relative] = digest(join(stage, relative));
      manifest.files_mode[relative] = status.mode & 0o777;
    }
    for (const [name, key] of [['run-worker-job.sh', 'runner_sha256'], ['prepare-build.mts', 'prepare_sha256']] as const) {
      add(name, readFileSync(join(ciDirectory, name)), 0o644);
      manifest[key] = digest(join(stage, name));
    }
    add('SOURCE.json', Buffer.from(JSON.stringify(manifest, null, 2)), 0o644);
    // Archive the exact staged bytes that were hashed; file lists are NUL-delimited and literal.
    execFileSync('tar', ['--create', '--gzip', '--file', resolve(archive), '--format=pax',
      `--mtime=@${snapshotTime}`, '--directory', stage, '--null', '--verbatim-files-from', '--no-recursion', '--files-from=-'],
    { input: Buffer.from(names.join('\0') + '\0'), stdio: ['pipe', 'inherit', 'inherit'] });
    return manifest;
  } finally { rmSync(stage, { recursive: true, force: true }); }
}

function main(): void {
  const { values } = parseArgs({ options: {
    source: { type: 'string', default: process.cwd() }, host: { type: 'string', default: 'doramcap-ci@100.64.0.20' },
    help: { type: 'boolean', short: 'h' },
  } });
  if (values.help) { console.log('Usage: node submit.mts [--source CHECKOUT] [--host SSH_HOST]'); return; }
  const job = '/home/doramcap-ci/jobs/' + randomUUID().replaceAll('-', '');
  const temporary = mkdtempSync(join(tmpdir(), 'doramcap-submit-'));
  try {
    const archive = join(temporary, 'source.tar.gz');
    createSnapshot(values.source, archive);
    const checksum = digest(archive);
    execFileSync('ssh', [...sshOptions, values.host, `mkdir -p ${job}`], { stdio: 'inherit' });
    execFileSync('scp', [...sshOptions, archive, `${values.host}:${job}/source.tar.gz`], { stdio: 'inherit' });
    const command = `cd ${job} && echo "${checksum}  source.tar.gz" | sha256sum -c - && `
      + 'tar -xzpf source.tar.gz && pueue add --label doramcap-native -- bash run-worker-job.sh';
    execFileSync('ssh', [...sshOptions, values.host, command], { stdio: 'inherit' });
    console.log(`Worker snapshot: ${values.host}:${job}`);
  } finally { rmSync(temporary, { recursive: true, force: true }); }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error((error as Error).message); process.exitCode = 1; }
}
