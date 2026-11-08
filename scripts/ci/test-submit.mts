// Verify actual archives from temporary Git checkouts without submitting remote jobs.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test, type TestContext } from 'node:test';
import { digest, prepare, type SnapshotManifest } from './prepare-build.mts';
import { createSnapshot } from './submit.mts';

function fixture(t: TestContext) {
  const root = mkdtempSync(join(tmpdir(), 'submit-ci-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const source = join(root, 'source');
  mkdirSync(source);
  const put = (name: string, data: string) => {
    mkdirSync(dirname(join(source, name)), { recursive: true });
    writeFileSync(join(source, name), data);
  };
  put('apps/system-monitor/tests/validate_native.ts', '// fixture');
  put('libs/resources/cgroup-bpf/examples/native_allocations.rs', '// fixture');
  put('.gitignore', '*.secret\n');
  put('Cargo.toml', '[workspace]');
  put('scripts/ci/check-boundaries.mts', '// checker');
  put('scripts/ci/tsconfig.json', '{}');
  execFileSync('git', ['init', '-q', source]);
  execFileSync('git', ['-C', source, 'add', '.']);
  execFileSync('git', ['-C', source, '-c', 'user.name=CI Fixture', '-c', 'user.email=ci@example.invalid',
    '-c', 'commit.gpgsign=false', 'commit', '-qm', 'fixture']);
  return { root, source, put, archive: join(root, 'source.tar.gz') };
}

test('archives preserve source bytes, modes, allowlists, dirty edits, and helper hashes', t => {
  // Arrange
  const { root, source, put, archive } = fixture(t);
  put('Cargo.toml', '[workspace]\n# dirty');
  put('apps/new file\nwith newline.ts', '// untracked');
  put('libs/run.sh', '#!/bin/sh\n');
  chmodSync(join(source, 'libs/run.sh'), 0o755);
  put('console/private.secret', 'ignored');
  put('.sudo-password', 'fixture exclusion');
  put('unrelated.txt', 'excluded');
  put('scripts/ci/unrelated.mts', 'excluded');
  // Act
  const submitted = createSnapshot(source, archive);
  const extracted = join(root, 'extracted');
  mkdirSync(extracted);
  execFileSync('tar', ['-xzpf', archive, '-C', extracted]);
  const manifest: SnapshotManifest = JSON.parse(readFileSync(join(extracted, 'SOURCE.json'), 'utf8'));
  prepare(extracted, join(root, 'cache'));
  // Assert
  assert.deepEqual(manifest, submitted);
  assert.equal(manifest.dirty, true);
  assert.match(manifest.head!, /^[a-f0-9]{40}$/);
  assert.equal(readFileSync(join(extracted, 'Cargo.toml'), 'utf8'), '[workspace]\n# dirty');
  assert.equal(readFileSync(join(extracted, 'apps/new file\nwith newline.ts'), 'utf8'), '// untracked');
  assert.equal(statSync(join(extracted, 'libs/run.sh')).mode & 0o777, 0o755);
  assert.equal(digest(join(extracted, 'prepare-build.mts')), manifest.prepare_sha256);
  assert.equal(digest(join(extracted, 'run-worker-job.sh')), manifest.runner_sha256);
  assert.equal(existsSync(join(extracted, 'scripts/ci/check-boundaries.mts')), true);
  assert.equal(existsSync(join(extracted, 'scripts/ci/tsconfig.json')), true);
  for (const excluded of ['.git', '.sudo-password', 'console/private.secret', 'unrelated.txt', 'scripts/ci/unrelated.mts']) {
    assert.equal(existsSync(join(extracted, excluded)), false, excluded);
  }
  const timestamps = Object.keys(manifest.files_sha256).map(name => statSync(join(extracted, name), { bigint: true }).mtimeNs);
  assert.equal(new Set(timestamps).size, 1);
});

test('tracked deletions are omitted and source symlinks are rejected', t => {
  // Arrange
  const { source, put, archive } = fixture(t);
  rmSync(join(source, 'Cargo.toml'));
  // Act / Assert
  assert.equal(Object.hasOwn(createSnapshot(source, archive).files_sha256, 'Cargo.toml'), false);
  put('libs/target.rs', '// target');
  symlinkSync('target.rs', join(source, 'libs/link.rs'));
  assert.throws(() => createSnapshot(source, archive), /Refusing non-regular source file/);
  rmSync(join(source, 'libs/target.rs'));
  assert.throws(() => createSnapshot(source, archive), /Refusing non-regular source file/);
});
