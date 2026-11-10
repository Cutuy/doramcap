import { createHash } from 'node:crypto';
// Verify actual archives from temporary Git checkouts without submitting remote jobs.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test, type TestContext } from 'node:test';
import { PREPARE_PATH, type SnapshotManifest } from './prepare-build.mts';
import { createSnapshot } from './submit.mts';

const digest = (path: string) => createHash('sha256').update(readFileSync(path)).digest('hex');

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
  put(PREPARE_PATH, readFileSync(new URL('./prepare-build.mts', import.meta.url), 'utf8'));
  put('scripts/ci/package.json', '{}');
  execFileSync('git', ['init', '-q', source]);
  execFileSync('git', ['-C', source, 'add', '.']);
  execFileSync('git', ['-C', source, '-c', 'user.name=CI Fixture', '-c', 'user.email=ci@example.invalid',
    '-c', 'commit.gpgsign=false', 'commit', '-qm', 'fixture']);
  return { root, source, put, archive: join(root, 'source.tar.gz') };
}

test('archives preserve source bytes, modes, allowlists, dirty edits, and helper hashes', async t => {
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
  for (const name of ['submit-run-only.mts', 'test-run-only.mts', 'run-only-worker.mts', 'worker-process.mts', 'worker-artifacts.mts', 'worker-ssh.mts', 'install-node.mts', 'install-nextest.mts', 'test-worker-runtime.mts', 'run-only-artifacts.txt']) {
    put(`scripts/ci/${name}`, '// run-only worker fixture');
  }
  // Act
  const submitted = await createSnapshot(source, archive);
  const extracted = join(root, 'extracted');
  mkdirSync(extracted);
  execFileSync('tar', ['-xzpf', archive, '-C', extracted]);
  const manifest: SnapshotManifest = JSON.parse(readFileSync(join(extracted, 'SOURCE.json'), 'utf8'));
  // Bootstrap must run before npm installs any tooling dependencies.
  execFileSync(process.execPath, [join(extracted, PREPARE_PATH), extracted, join(root, 'cache')]);
  // Assert
  assert.deepEqual(manifest, submitted.manifest);
  assert.equal(submitted.checksum, digest(archive));
  assert.equal(manifest.dirty, true);
  assert.match(manifest.head!, /^[a-f0-9]{40}$/);
  assert.equal(readFileSync(join(extracted, 'Cargo.toml'), 'utf8'), '[workspace]\n# dirty');
  assert.equal(readFileSync(join(extracted, 'apps/new file\nwith newline.ts'), 'utf8'), '// untracked');
  assert.equal(statSync(join(extracted, 'libs/run.sh')).mode & 0o777, 0o755);
  assert.equal(digest(join(extracted, PREPARE_PATH)), manifest.prepare_sha256);
  assert.equal(digest(join(extracted, 'run-worker-job.sh')), manifest.runner_sha256);
  assert.equal(existsSync(join(extracted, 'scripts/ci/check-boundaries.mts')), true);
  assert.equal(existsSync(join(extracted, 'scripts/ci/package.json')), true);
  for (const name of ['submit-run-only.mts', 'test-run-only.mts', 'run-only-worker.mts', 'worker-process.mts', 'worker-artifacts.mts', 'worker-ssh.mts', 'install-node.mts', 'install-nextest.mts', 'test-worker-runtime.mts', 'run-only-artifacts.txt']) {
    assert.equal(existsSync(join(extracted, 'scripts/ci', name)), true, name);
  }
  for (const excluded of ['.git', '.sudo-password', 'console/private.secret', 'unrelated.txt', 'scripts/ci/unrelated.mts']) {
    assert.equal(existsSync(join(extracted, excluded)), false, excluded);
  }
  const timestamps = Object.keys(manifest.files_sha256).map(name => statSync(join(extracted, name), { bigint: true }).mtimeNs);
  assert.equal(new Set(timestamps).size, 1);
});

test('tracked deletions are omitted and source symlinks are rejected', async t => {
  // Arrange
  const { source, put, archive } = fixture(t);
  rmSync(join(source, 'Cargo.toml'));
  // Act / Assert
  assert.equal(Object.hasOwn((await createSnapshot(source, archive)).manifest.files_sha256, 'Cargo.toml'), false);
  put('libs/target.rs', '// target');
  symlinkSync('target.rs', join(source, 'libs/link.rs'));
  await assert.rejects(createSnapshot(source, archive), /Refusing non-regular source file/);
  rmSync(join(source, 'libs/target.rs'));
  await assert.rejects(createSnapshot(source, archive), /Refusing non-regular source file/);
});
