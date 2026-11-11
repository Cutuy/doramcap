#!/usr/bin/env node
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { chmodSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { execaSync } from 'execa';
import { minimatch } from 'minimatch';
import { PREPARE_PATH, RUNNER_PATH, type SnapshotManifest } from './prepare-build.mts';
const included = ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.node-version', 'libs/**', 'apps/**', 'console/**',
  'scripts/ci/{check-boundaries,test-boundaries,dora-release,test-dora-release,test-node,test-mcap,prepare-build,test-prepare-build,snapshot,test-snapshot,hash-file,worker-process,worker-ssh,install-node,test-worker-runtime}.mts',
  'scripts/ci/{wsl2,macos-linux}/**/*.mts',
  'scripts/ci/{boundaries.json,package.json,package-lock.json,tsconfig.json,macos-linux/artifacts.txt}'];
const hash = (data: Uint8Array) => createHash('sha256').update(data).digest('hex');
/** Stage exact source bytes and permissions; tar gives every entry one timestamp. Always remove staging files. */
export async function createSnapshot(source: string, archive: string): Promise<{ manifest: SnapshotManifest; checksum: string }> {
  source = realpathSync(source);
  for (const name of ['apps/system-monitor/tests/validate_native.ts', 'libs/resources/cgroup-bpf/examples/native_allocations.rs', PREPARE_PATH, RUNNER_PATH]) {
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
    manifest.runner_sha256 = manifest.files_sha256[RUNNER_PATH];
    manifest.prepare_sha256 = manifest.files_sha256[PREPARE_PATH];
    writeFileSync(join(staged, 'SOURCE.json'), JSON.stringify(manifest), { mode: 0o644 });
    execaSync('tar', ['-czf', resolve(archive), `--mtime=@${Math.floor(Date.now() / 1000)}`, '-C', staged, '.']);
    return { manifest, checksum: hash(readFileSync(archive)) };
  } finally { rmSync(staged, { recursive: true, force: true }); }
}
