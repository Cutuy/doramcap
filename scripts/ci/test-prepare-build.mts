// Exercise source-cache correctness with the real rsync executable.
import assert from 'node:assert/strict';
import { chmodSync, existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, renameSync, rmSync, statSync, symlinkSync, unlinkSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test, type TestContext } from 'node:test';
import { digest, prepare, type SnapshotManifest } from './prepare-build.mts';

function fixture(t: TestContext) {
  const root = mkdtempSync(join(tmpdir(), 'prepare-build-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const cache = join(root, 'cache');
  const snapshot = (name: string, sources: Record<string, string>) => {
    const directory = join(root, name);
    mkdirSync(directory);
    const manifest: SnapshotManifest = { files_sha256: {}, files_mode: {}, runner_sha256: '', prepare_sha256: '' };
    for (const [relative, data] of Object.entries(sources)) {
      const path = join(directory, relative);
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, data);
      chmodSync(path, 0o644);
      manifest.files_sha256[relative] = digest(path);
      manifest.files_mode[relative] = 0o644;
    }
    for (const [relative, key] of [['run-worker-job.sh', 'runner_sha256'], ['prepare-build.mts', 'prepare_sha256']] as const) {
      const path = join(directory, relative);
      writeFileSync(path, 'runner helper');
      manifest[key] = digest(path);
    }
    writeFileSync(join(directory, 'SOURCE.json'), JSON.stringify(manifest));
    writeFileSync(join(directory, 'source.tar.gz'), 'archive remains in snapshot');
    return directory;
  };
  return { root, cache, snapshot };
}

test('identical files retain mtimes; edits, deletes, renames, and generated files synchronize', t => {
  // Arrange
  const { cache, snapshot } = fixture(t);
  const first = snapshot('first', { 'libs/a.rs': 'old', 'libs/gone.rs': 'gone', 'tests/native.py': 'pass' });
  prepare(first, cache);
  const cached = join(cache, 'libs/a.rs');
  utimesSync(cached, 1, 1);
  const identical = snapshot('identical', { 'libs/a.rs': 'old', 'libs/gone.rs': 'gone', 'tests/native.py': 'pass' });
  // Act
  prepare(identical, cache);
  // Assert
  assert.equal(statSync(cached, { bigint: true }).mtimeNs, 1_000_000_000n);
  assert.equal(existsSync(join(cache, 'source.tar.gz')), false);
  assert.equal(statSync(join(cache, 'tests/__pycache__')).uid, process.getuid!());
  // Arrange: a same-size edit with an old source timestamp still updates.
  const edited = snapshot('edited', { 'libs/a.rs': 'new', 'libs/renamed.rs': 'gone' });
  utimesSync(join(edited, 'libs/a.rs'), 1, 1);
  writeFileSync(join(cache, 'generated.tmp'), 'stale');
  // Act
  prepare(edited, cache);
  // Assert
  assert.equal(readFileSync(cached, 'utf8'), 'new');
  assert.ok(statSync(cached, { bigint: true }).mtimeNs > 1_000_000_000n);
  assert.equal(existsSync(join(cache, 'libs/gone.rs')), false);
  assert.equal(existsSync(join(cache, 'libs/renamed.rs')), true);
  assert.equal(existsSync(join(cache, 'generated.tmp')), false);
  assert.equal(readFileSync(join(first, 'libs/a.rs'), 'utf8'), 'old');
});

test('cached symlinks are replaced without following their targets', t => {
  // Arrange
  const { root, cache, snapshot } = fixture(t);
  const source = snapshot('snapshot', { 'apps/frontend/package.json': 'package' });
  prepare(source, cache);
  const outside = join(root, 'outside');
  mkdirSync(outside);
  const sentinel = join(outside, 'sentinel');
  writeFileSync(sentinel, 'keep');
  const modules = join(cache, 'apps/frontend/node_modules');
  mkdirSync(join(modules, '.bin'), { recursive: true });
  symlinkSync('../tsx/cli.mjs', join(modules, '.bin/tsx'));
  symlinkSync(outside, join(modules, 'external'));
  unlinkSync(join(cache, 'SOURCE.json'));
  symlinkSync(sentinel, join(cache, 'SOURCE.json'));
  renameSync(join(cache, 'apps/frontend'), join(cache, 'old-frontend'));
  symlinkSync(outside, join(cache, 'apps/frontend'));
  // Act
  prepare(source, cache);
  // Assert
  assert.equal(existsSync(join(cache, 'old-frontend')), false);
  assert.equal(lstatSync(join(cache, 'apps/frontend')).isSymbolicLink(), false);
  assert.equal(lstatSync(join(cache, 'SOURCE.json')).isSymbolicLink(), false);
  assert.equal(readFileSync(join(cache, 'apps/frontend/package.json'), 'utf8'), 'package');
  assert.equal(readFileSync(sentinel, 'utf8'), 'keep');
});

test('permission-only changes retain mtimes', t => {
  // Arrange
  const { cache, snapshot } = fixture(t);
  prepare(snapshot('first', { 'run.sh': 'echo ok' }), cache);
  const cached = join(cache, 'run.sh');
  const before = statSync(cached, { bigint: true }).mtimeNs;
  const second = snapshot('second', { 'run.sh': 'echo ok' });
  chmodSync(join(second, 'run.sh'), 0o755);
  const manifest: SnapshotManifest = JSON.parse(readFileSync(join(second, 'SOURCE.json'), 'utf8'));
  manifest.files_mode['run.sh'] = 0o755;
  writeFileSync(join(second, 'SOURCE.json'), JSON.stringify(manifest));
  // Act
  prepare(second, cache);
  // Assert
  assert.equal(statSync(cached).mode & 0o777, 0o755);
  assert.equal(statSync(cached, { bigint: true }).mtimeNs, before);
});

for (const corruption of ['content', 'extra', 'symlink', 'runner', 'mode', 'path', 'permissions']) {
  test(`invalid snapshot (${corruption}) preserves cache`, t => {
    // Arrange
    const { cache, snapshot } = fixture(t);
    const source = snapshot('snapshot', { 'src.rs': 'valid' });
    mkdirSync(cache);
    const sentinel = join(cache, 'sentinel');
    writeFileSync(sentinel, 'keep');
    if (corruption === 'content') writeFileSync(join(source, 'src.rs'), 'tampered');
    else if (corruption === 'extra') writeFileSync(join(source, 'extra'), 'unlisted');
    else if (corruption === 'symlink') symlinkSync(join(source, 'src.rs'), join(source, 'link'));
    else if (corruption === 'runner') writeFileSync(join(source, 'run-worker-job.sh'), 'tampered');
    else if (corruption === 'mode') chmodSync(join(source, 'src.rs'), 0o755);
    else {
      const manifest: SnapshotManifest = JSON.parse(readFileSync(join(source, 'SOURCE.json'), 'utf8'));
      if (corruption === 'path') manifest.files_sha256['../outside'] = digest(sentinel);
      else delete manifest.files_mode['src.rs'];
      writeFileSync(join(source, 'SOURCE.json'), JSON.stringify(manifest));
    }
    // Act / Assert
    assert.throws(() => prepare(source, cache));
    assert.equal(readFileSync(sentinel, 'utf8'), 'keep');
  });
}

test('symlink destinations and overlapping paths are rejected', t => {
  // Arrange
  const { root, cache, snapshot } = fixture(t);
  const source = snapshot('snapshot', { 'src.rs': 'valid' });
  const outside = join(root, 'outside');
  mkdirSync(outside);
  symlinkSync(outside, cache);
  // Act / Assert
  assert.throws(() => prepare(source, cache), /real directories/);
  assert.deepEqual(readdirSync(outside), []);
  assert.throws(() => prepare(source, source), /non-overlapping/);
  assert.throws(() => prepare(source, join(source, 'child')), /non-overlapping/);
  assert.throws(() => prepare(source, root), /non-overlapping/);
  const alias = join(root, 'alias');
  symlinkSync(source, alias);
  assert.throws(() => prepare(source, join(alias, 'child')), /non-overlapping/);
});
