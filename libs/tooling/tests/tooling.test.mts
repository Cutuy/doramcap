import assert from 'node:assert/strict';
import { existsSync, mkdirSync, readFileSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { writeTarGzip } from '../archive.mts';
import { canonicalPath, digest, sha256, withTempDirectory } from '../fs.mts';
import { globMatcher } from '../glob.mts';
import { capture } from '../process.mts';

test('process capture retains literal argv and output, and reports exit/signal/spawn failures', () => {
  // Arrange
  const argument = 'space ; $not-a-shell';
  // Act / Assert
  assert.equal(capture(process.execPath, ['-e', 'console.log(process.argv[1])', argument]).stdout, argument + '\n');
  assert.throws(() => capture(process.execPath, ['-e', 'process.exit(7)']), (error: any) => error.exitCode === 7);
  assert.throws(() => capture(process.execPath, ['-e', 'process.kill(process.pid, "SIGTERM")']), (error: any) => error.signal === 'SIGTERM');
  assert.throws(() => capture('/nonexistent/doramcap-tool'), (error: any) => error.code === 'ENOENT');
});

test('temporary directories live through asynchronous callbacks and are removed on success and failure', async () => {
  // Arrange
  let directory = '';
  const failure = new Error('callback failed');
  // Act
  const result = await withTempDirectory('tooling-success-', async path => {
    directory = path;
    await Promise.resolve();
    assert.equal(existsSync(path), true);
    return 'value';
  });
  // Assert
  assert.equal(result, 'value');
  assert.equal(existsSync(directory), false);
  // Act / Assert
  await assert.rejects(withTempDirectory('tooling-failure-', async path => {
    directory = path;
    writeFileSync(join(path, 'output'), 'partial');
    throw failure;
  }), error => error === failure);
  assert.equal(existsSync(directory), false);
});

test('canonical paths retain missing children after resolving existing symlinks', async () => {
  await withTempDirectory('tooling-path-', root => {
    // Arrange
    const directory = join(root, 'directory');
    mkdirSync(directory);
    symlinkSync(directory, join(root, 'alias'));
    writeFileSync(join(root, 'file'), 'data');
    // Act / Assert
    assert.equal(canonicalPath(join(root, 'alias/missing/child')), join(directory, 'missing/child'));
    assert.equal(canonicalPath('/'), '/');
    assert.throws(() => canonicalPath(join(root, 'file/child')));
  });
});

test('glob groups use case-sensitive minimatch syntax and include dotfiles', () => {
  // Arrange
  const included = globMatcher(['libs/**', 'workload-*', 'adapter-[ab]', '.node-version']);
  // Act / Assert
  for (const name of ['libs/tooling/fs.mts', 'libs/.hidden', 'workload-docker', 'adapter-a', '.node-version']) assert.equal(included(name), true);
  for (const name of ['Libs/tooling/fs.mts', 'adapter-c', 'outside.txt']) assert.equal(included(name), false);
  assert.equal(globMatcher(['[a--!]'])('b'), false);
});

test('archives round-trip bytes, modes, Unicode and long/newline names through system tar', async () => {
  await withTempDirectory('tooling-tar-', async root => {
    // Arrange
    const name = 'long/'.repeat(30) + 'new file\nλ.ts';
    const data = Buffer.from('source\0bytes');
    const archive = join(root, 'source.tar.gz');
    const timestamp = new Date(1000);
    // Act
    await writeTarGzip(archive, [{ name, data, mode: 0o755 }], timestamp);
    capture('tar', ['-xzpf', archive, '-C', root]);
    // Assert
    assert.deepEqual(readFileSync(join(root, name)), data);
    assert.equal(statSync(join(root, name)).mode & 0o777, 0o755);
    assert.equal(statSync(join(root, name)).mtimeMs, timestamp.getTime());
    assert.equal(digest(join(root, name)), sha256(data));
  });
});

test('archive input and output errors reject after stream cleanup', async () => {
  await withTempDirectory('tooling-tar-errors-', async root => {
    // Arrange
    const failure = new Error('input failed');
    function* broken() {
      yield { name: 'first', data: Buffer.from('partial'), mode: 0o644 };
      throw failure;
    }
    // Act / Assert
    await assert.rejects(writeTarGzip(join(root, 'partial.tar.gz'), broken()), error => error === failure);
    await assert.rejects(writeTarGzip(join(root, 'missing/output.tar.gz'), [
      { name: 'first', data: Buffer.alloc(1024), mode: 0o644 },
    ]), { code: 'ENOENT' });
  });
});
