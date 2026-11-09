import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import {
  chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync,
  symlinkSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import type { TestContext } from 'node:test';

const SCRIPT = fileURLToPath(new URL('./pool.ts', import.meta.url));
const OWNER = 'test-owner';
const BRANCH = 'codex/test';
const PR_URL = 'https://github.com/example/repo/pull/1';
type Result = { status: number | null; stdout: string; stderr: string };
type Slot = { id: number; path: string; lease: { owner: string; token: string; branch: string; state: string } };

function fixture(t: TestContext) {
  const temp = mkdtempSync(join(tmpdir(), 'worktree-pool-'));
  t.after(() => rmSync(temp, { recursive: true, force: true }));
  const repo = join(temp, 'repo');
  mkdirSync(repo);
  const git = (root: string, ...args: string[]) => {
    const result = spawnSync('git', ['-C', root, ...args], { encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout.trim();
  };
  const write = (root: string, rel: string, text = 'preserve or clean') => {
    const path = join(root, rel);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, text);
    return path;
  };
  git(repo, 'init', '-b', 'main');
  git(repo, 'config', 'user.name', 'Pool Test');
  git(repo, 'config', 'user.email', 'pool@example.com');
  git(repo, 'config', 'commit.gpgsign', 'false');
  write(repo, '.gitignore', 'target/\napps/*/frontend/dist/\nnode_modules/\n.env\n*.mcap\n');
  write(repo, 'source', 'initial');
  git(repo, 'add', '.');
  git(repo, 'commit', '-m', 'Initial');
  const bin = join(temp, 'bin');
  mkdirSync(bin);
  const env = { ...process.env, PATH: `${bin}:${process.env.PATH}` };
  const args = (...rest: string[]) => [SCRIPT, '--repo', repo, ...rest];
  const command = (...rest: string[]): Result => spawnSync(process.execPath, args(...rest), { encoding: 'utf8', env });
  const ok = (...rest: string[]) => {
    const result = command(...rest);
    assert.equal(result.status, 0, result.stderr);
    return JSON.parse(result.stdout);
  };
  const start = (...rest: string[]) => {
    const child = spawn(process.execPath, args(...rest), { env, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '';
    child.stdout.on('data', chunk => { stdout += chunk; });
    child.stderr.on('data', chunk => { stderr += chunk; });
    const done = once(child, 'close').then(([status]) => ({ status, stdout, stderr }) as Result);
    return { child, done };
  };
  const state = () => JSON.parse(readFileSync(join(repo, '.git/agent-pool/state.json'), 'utf8'));
  const acquire = (owner = OWNER, branch = BRANCH): Slot => ok('acq', '--owner', owner, '--branch', branch);
  const releaseArgs = (slot: Slot) => ['release', '--owner', slot.lease.owner, '--token', slot.lease.token, '--main'];
  const commit = (root: string) => { git(root, 'add', '.'); git(root, 'commit', '-m', 'Feature'); return git(root, 'rev-parse', 'HEAD'); };
  const mockPr = (slot: Slot, overrides = {}, extra = '') => {
    const pr = { url: PR_URL, state: 'OPEN', headRefName: slot.lease.branch,
      headRefOid: git(slot.path, 'rev-parse', 'HEAD'), ...overrides };
    const script = write(bin, 'gh', `#!${process.execPath}\n${extra}\nconsole.log(${JSON.stringify(JSON.stringify(pr))});\n`);
    chmodSync(script, 0o755);
  };
  ok('init');
  return { temp, repo, git, write, command, ok, start, state, acquire, releaseArgs, commit, mockPr };
}

test('ownership: owner and token are required, resume preserves work, status hides tokens', t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  const source = f.write(slot.path, 'unfinished.ts');

  // Act / Assert
  const before = f.state();
  for (const [owner, token] of [['someone-else', slot.lease.token], [OWNER, 'wrong-token']]) {
    assert.match(f.command('release', '--owner', owner, '--token', token, '--main').stderr, /owner and token/);
    assert.match(f.command('acq', '--owner', owner, '--branch', BRANCH, '--token', token).stderr, /owner and token/);
  }
  assert.notEqual(f.command('release', '--token', slot.lease.token, '--main').status, 0);
  assert.match(f.command('acq', '--owner', OWNER, '--branch', 'codex/another').stderr, /already has a lease/);
  assert.match(f.command('acq', '--owner', OWNER, '--branch', 'codex/another', '--token', slot.lease.token).stderr, /leased branch/);
  assert.deepEqual(f.ok('acq', '--owner', OWNER, '--branch', BRANCH, '--token', slot.lease.token), slot);
  assert.deepEqual(f.state(), before);
  assert.equal(readFileSync(source, 'utf8'), 'preserve or clean');
  assert.ok(!f.command('status').stdout.includes(slot.lease.token));
});

test('simultaneous acquisition: five unique owners win and excess contenders fail', async t => {
  // Arrange
  const f = fixture(t);

  // Act
  const results = await Promise.all(Array.from({ length: 8 }, (_, id) =>
    f.start('acq', '--owner', `owner-${id}`, '--branch', `codex/contender-${id}`).done));

  // Assert
  const wins = results.filter(result => result.status === 0).map(result => JSON.parse(result.stdout));
  assert.equal(wins.length, 5, JSON.stringify(results));
  assert.equal(new Set(wins.map(slot => slot.id)).size, 5);
  assert.equal(new Set(wins.map(slot => slot.lease.token)).size, 5);
  for (const loss of results.filter(result => result.status !== 0)) assert.match(loss.stderr, /Pool exhausted/);
  assert.equal(f.state().slots.filter((slot: Slot) => slot.lease).length, 5);
});

test('same-owner acquisition contention creates exactly one lease', async t => {
  // Arrange
  const f = fixture(t);

  // Act
  const results = await Promise.all([0, 1].map(id => f.start('acq', '--owner', OWNER, '--branch', `codex/same-${id}`).done));

  // Assert
  assert.equal(results.filter(result => result.status === 0).length, 1);
  assert.match(results.find(result => result.status !== 0)!.stderr, /already has a lease/);
  assert.equal(f.state().slots.filter((slot: Slot) => slot.lease).length, 1);
});

test('simultaneous release and acquisition preserve the new owner and reject stale release', async t => {
  // Arrange
  const f = fixture(t), original = f.acquire();

  // Act
  const results = await Promise.all([
    f.start(...f.releaseArgs(original)).done,
    f.start(...f.releaseArgs(original)).done,
    f.start('acq', '--owner', 'next', '--branch', 'codex/next').done,
  ]);

  // Assert
  assert.equal(results.slice(0, 2).filter(result => result.status === 0).length, 1);
  assert.equal(results[2].status, 0, results[2].stderr);
  const next = JSON.parse(results[2].stdout);
  assert.notEqual(next.lease.token, original.lease.token);
  assert.match(f.command(...f.releaseArgs(original)).stderr, /owner and token/);
  assert.equal(f.state().slots.find((slot: Slot) => slot.id === next.id).lease.token, next.lease.token);
});

test('snapshot copies tracked and untracked edits while preserving main and ignored data', t => {
  // Arrange
  const f = fixture(t);
  f.write(f.repo, 'source', 'staged');
  f.git(f.repo, 'add', 'source');
  f.write(f.repo, 'source', 'staged plus unstaged');
  f.write(f.repo, 'untracked', 'untracked content');
  symlinkSync('untracked', join(f.repo, 'link'));
  f.write(f.repo, '.env', 'local config');
  const output = f.write(f.repo, 'console/server/dist/generated');
  const before = f.git(f.repo, 'status', '--porcelain');

  // Act
  const slot = f.acquire();

  // Assert
  assert.equal(f.git(slot.path, 'diff', '--binary', 'HEAD'), f.git(f.repo, 'diff', '--binary', 'HEAD'));
  assert.equal(readFileSync(join(slot.path, 'untracked'), 'utf8'), 'untracked content');
  assert.equal(readFileSync(join(slot.path, 'link'), 'utf8'), 'untracked content');
  assert.ok(!existsSync(join(slot.path, '.env')));
  assert.ok(!existsSync(join(slot.path, 'console/server/dist')));
  assert.ok(existsSync(output));
  assert.equal(f.git(f.repo, 'status', '--porcelain'), before);
});

test('release requires committed and integrated source, then cleans outputs and keeps caches', t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  const source = f.write(slot.path, 'source', 'feature');
  const artifact = f.write(slot.path, 'target/artifact');
  const preserved = ['node_modules/package/cache', '.env', 'recordings/session.mcap'].map(rel => f.write(slot.path, rel));

  // Act / Assert
  assert.match(f.command(...f.releaseArgs(slot)).stderr, /uncommitted/);
  assert.ok(!existsSync(artifact));
  assert.equal(readFileSync(source, 'utf8'), 'feature');
  const commit = f.commit(slot.path);
  assert.match(f.command(...f.releaseArgs(slot)).stderr, /unintegrated/);
  f.git(f.repo, 'cherry-pick', commit);
  const outputs = ['frontend/dist', 'apps/demo/frontend/dist', 'apps/demo/frontend/generated',
    'apps/demo/frontend/src/generated', 'apps/demo/frontend/src/.generated',
    'apps/demo/frontend/.vite', 'apps/demo/frontend/.vite-temp', 'apps/demo/frontend/test-results',
    'apps/demo/frontend/playwright-report', 'apps/project-explorer/frontend/public/content',
    'apps/cuda-shm-demo/build', 'console/server/build', 'console/server/dist', 'console/server/src/generated'];
  for (const output of outputs) f.write(slot.path, `${output}/artifact`);
  f.ok(...f.releaseArgs(slot));
  assert.equal(f.state().slots[0].lease, null);
  assert.equal(f.git(slot.path, 'branch', '--show-current'), '');
  assert.equal(f.git(slot.path, 'rev-parse', 'HEAD'), commit);
  assert.equal(f.git(f.repo, 'branch', '--list', BRANCH), '');
  assert.equal(f.git(f.repo, 'branch', '--show-current'), 'main');
  for (const output of outputs) assert.ok(!existsSync(join(slot.path, output)));
  for (const path of preserved) assert.equal(readFileSync(path, 'utf8'), 'preserve or clean');
});

test('acquisition skips dirty free slots, cleans outputs and preserves ignored files', t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  f.ok(...f.releaseArgs(slot));
  const dirty = f.write(slot.path, 'unfinished.ts');
  const second = join(dirname(slot.path), '2');
  const cache = f.write(second, 'node_modules/package/cache');
  const output = f.write(second, 'console/server/dist/artifact');

  // Act
  const next = f.acquire('next', 'codex/next');

  // Assert
  assert.equal(next.id, 2);
  assert.ok(existsSync(dirty));
  assert.ok(existsSync(cache));
  assert.ok(!existsSync(output));
});

test('all dirty slots require explicit discard approval and remain untouched', t => {
  // Arrange
  const f = fixture(t);
  const paths = f.state().slots.map((slot: Slot) => f.write(slot.path, 'unfinished.ts'));

  // Act
  const result = f.command('acq', '--owner', OWNER, '--branch', BRANCH);

  // Assert
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Obtain user approval/);
  assert.ok(f.state().slots.every((slot: Slot) => !slot.lease));
  for (const path of paths) assert.ok(existsSync(path));
});

test('failed preparation retains its owner and token and preserves ignored collisions', t => {
  // Arrange
  const f = fixture(t), first = f.state().slots[0];
  const config = f.write(first.path, '.env', 'slot config');
  f.write(f.repo, '.env', 'main config');
  f.git(f.repo, 'add', '-f', '.env');
  f.git(f.repo, 'commit', '-m', 'Track fixture');

  // Act
  const result = f.command('acq', '--owner', OWNER, '--branch', BRANCH);

  // Assert
  assert.notEqual(result.status, 0);
  const slot = f.state().slots[0];
  assert.equal(slot.lease.state, 'preparing');
  assert.equal(slot.lease.owner, OWNER);
  assert.ok(result.stderr.includes(slot.lease.token));
  assert.equal(readFileSync(config, 'utf8'), 'slot config');
  assert.match(f.command(...f.releaseArgs(slot)).stderr, /manual repair/);
  assert.equal(f.acquire('next', 'codex/next').id, 2);
});

test('release refuses a switched branch before cleaning anything', t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  const artifact = f.write(slot.path, 'target/artifact');
  f.git(slot.path, 'checkout', '-b', 'codex/switched');

  // Act
  const result = f.command(...f.releaseArgs(slot));

  // Assert
  assert.match(result.stderr, /differs from leased branch/);
  assert.ok(existsSync(artifact));
  assert.equal(f.state().slots[0].lease.token, slot.lease.token);
});

test('symlinked output and ancestor paths cannot delete external files', t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  const external = f.write(f.temp, 'external/artifact');
  symlinkSync(dirname(external), join(slot.path, 'target'));

  // Act / Assert
  assert.match(f.command(...f.releaseArgs(slot)).stderr, /symlink/);
  assert.ok(existsSync(external));
  rmSync(join(slot.path, 'target'));
  symlinkSync(f.temp, join(slot.path, 'apps'));
  assert.match(f.command(...f.releaseArgs(slot)).stderr, /symlink/);
  assert.ok(existsSync(external));
  assert.equal(f.state().slots[0].lease.token, slot.lease.token);
});

test('tracked output blocks all cleanup and retains the lease', t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  const tracked = f.write(slot.path, 'target/fixture');
  const output = f.write(slot.path, 'console/server/dist/artifact');
  f.git(slot.path, 'add', '-f', 'target/fixture');

  // Act
  const result = f.command(...f.releaseArgs(slot));

  // Assert
  assert.match(result.stderr, /tracked files/);
  assert.ok(existsSync(tracked));
  assert.ok(existsSync(output));
  assert.equal(f.state().slots[0].lease.token, slot.lease.token);
});

test('cleanup failure retains the lease and succeeds after repair', { skip: process.getuid?.() === 0 }, t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  const artifact = f.write(slot.path, 'apps/demo/frontend/dist/artifact');
  const blocked = join(slot.path, 'apps');
  chmodSync(blocked, 0);

  // Act
  let result: Result;
  try { result = f.command(...f.releaseArgs(slot)); }
  finally { chmodSync(blocked, 0o755); }

  // Assert
  assert.notEqual(result.status, 0);
  assert.ok(existsSync(artifact));
  assert.equal(f.state().slots[0].lease.token, slot.lease.token);
  f.ok(...f.releaseArgs(slot));
  assert.equal(f.state().slots[0].lease, null);
});

test('publication release verifies the exact PR, deletes the local branch and preserves the remote', t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  f.write(slot.path, 'source', 'published feature');
  const published = f.commit(slot.path);
  const remote = join(f.temp, 'remote.git');
  f.git(f.repo, 'init', '--bare', remote);
  f.git(f.repo, 'remote', 'add', 'origin', remote);
  f.git(slot.path, 'push', '--set-upstream', 'origin', BRANCH);
  const remoteBefore = f.git(f.repo, 'ls-remote', '--heads', 'origin');
  const args = ['release', '--owner', OWNER, '--token', slot.lease.token, '--pr', PR_URL];

  // Act / Assert
  for (const mismatch of [{ url: `${PR_URL}0` }, { state: 'CLOSED' }, { headRefName: 'other' }, { headRefOid: 'stale' }]) {
    f.mockPr(slot, mismatch);
    assert.match(f.command(...args).stderr, /PR must be published/);
    assert.equal(f.state().slots[0].lease.token, slot.lease.token);
  }
  f.mockPr(slot);
  f.ok(...args);
  assert.equal(f.state().slots[0].lease, null);
  assert.equal(f.state().slots[0].last_pr, PR_URL);
  assert.equal(f.git(slot.path, 'branch', '--show-current'), '');
  assert.equal(f.git(slot.path, 'rev-parse', 'HEAD'), published);
  assert.equal(f.git(f.repo, 'branch', '--list', BRANCH), '');
  assert.equal(f.git(f.repo, 'ls-remote', '--heads', 'origin'), remoteBefore);
  assert.equal(f.git(f.repo, 'rev-parse', `refs/remotes/origin/${BRANCH}`), published);
});


test('main cannot be acquired or released, including a legacy main lease', t => {
  // Arrange
  const f = fixture(t);
  f.git(f.repo, 'branch', '-m', 'primary');

  // Act / Assert: main is forbidden even when that branch does not exist yet.
  assert.match(f.command('acq', '--owner', OWNER, '--branch', 'main').stderr, /Cannot lease main/);
  const slot = f.acquire();
  f.git(slot.path, 'branch', '-m', 'main');
  const state = f.state();
  state.slots[0].lease.branch = 'main';
  f.write(f.repo, '.git/agent-pool/state.json', JSON.stringify(state));
  const originalHead = f.git(slot.path, 'rev-parse', 'HEAD');
  const artifact = f.write(slot.path, 'target/artifact');

  // Act / Assert: a legacy lease cannot delete main or clean its checkout.
  for (const target of [['--main'], ['--pr', PR_URL]]) {
    const result = f.command('release', '--owner', OWNER, '--token', slot.lease.token, ...target);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Cannot release or delete main/);
    assert.equal(f.git(slot.path, 'branch', '--show-current'), 'main');
    assert.equal(f.git(slot.path, 'rev-parse', 'refs/heads/main'), originalHead);
    assert.ok(existsSync(artifact));
    assert.equal(f.state().slots[0].lease.token, slot.lease.token);
  }
});

test('failed local branch deletion restores the checkout and retains the lease for retry', t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  const originalHead = f.git(slot.path, 'rev-parse', 'HEAD');
  const lock = f.write(f.repo, `.git/refs/heads/${BRANCH}.lock`, '');

  // Act
  const result = f.command(...f.releaseArgs(slot));

  // Assert
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /cannot lock ref/);
  assert.equal(f.git(slot.path, 'branch', '--show-current'), BRANCH);
  assert.equal(f.git(slot.path, 'rev-parse', 'HEAD'), originalHead);
  assert.equal(f.state().slots[0].lease.token, slot.lease.token);

  // Act / Assert: removing the competing lock lets the same lease finish release.
  rmSync(lock);
  f.ok(...f.releaseArgs(slot));
  assert.equal(f.state().slots[0].lease, null);
  assert.equal(f.git(slot.path, 'branch', '--show-current'), '');
  assert.equal(f.git(f.repo, 'branch', '--list', BRANCH), '');
});

for (const target of ['main', 'pr']) {
  for (const deleted of [false, true]) {
    test(`detached ${target} release requires confirmation with branch ${deleted ? 'deleted' : 'present'}`, t => {
      // Arrange: reproduce either interruption boundary using real Git state.
      const f = fixture(t), slot = f.acquire();
      f.write(slot.path, 'source', 'verified feature');
      const feature = f.commit(slot.path);
      if (target === 'main') f.git(f.repo, 'cherry-pick', feature);
      else f.mockPr(slot);
      const args = ['release', '--owner', OWNER, '--token', slot.lease.token,
        ...(target === 'main' ? ['--main'] : ['--pr', PR_URL])];
      f.git(slot.path, 'checkout', '--detach');
      if (deleted) f.git(slot.path, 'branch', '-D', BRANCH);
      const artifact = f.write(slot.path, 'target/artifact');

      // Act / Assert: warning leaves the checkout and lease untouched.
      const result = f.command(...args);
      assert.notEqual(result.status, 0);
      assert.match(result.stderr, /WARNING.*detached/);
      assert.ok(result.stderr.includes(slot.path));
      assert.ok(result.stderr.includes(BRANCH));
      assert.ok(result.stderr.includes(feature));
      assert.match(result.stderr, /user confirmation.*--confirm-detached/);
      assert.ok(existsSync(artifact));
      assert.equal(f.state().slots[0].lease.token, slot.lease.token);
      assert.match(f.command('acq', '--owner', OWNER, '--branch', BRANCH,
        '--token', slot.lease.token).stderr, /differs from leased branch/);

      // Act / Assert: explicit confirmation completes release at the verified commit.
      f.ok(...args, '--confirm-detached');
      assert.equal(f.state().slots[0].lease, null);
      assert.equal(f.git(slot.path, 'branch', '--show-current'), '');
      assert.equal(f.git(slot.path, 'rev-parse', 'HEAD'), feature);
      assert.equal(f.git(f.repo, 'branch', '--list', BRANCH), '');
      assert.ok(!existsSync(artifact));
    });
  }
}

test('detached confirmation preserves ownership, cleanliness and publication checks', t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  f.write(slot.path, 'source', 'unpublished feature');
  const feature = f.commit(slot.path);
  f.git(slot.path, 'checkout', '--detach');
  const args = ['release', '--owner', OWNER, '--token', slot.lease.token, '--confirm-detached'];

  // Act / Assert
  assert.match(f.command('release', '--owner', 'other', '--token', slot.lease.token,
    '--confirm-detached', '--main').stderr, /owner and token/);
  const dirty = f.write(slot.path, 'unfinished.ts');
  assert.match(f.command(...args, '--main').stderr, /uncommitted/);
  rmSync(dirty);
  assert.match(f.command(...args, '--main').stderr, /unintegrated/);
  f.mockPr(slot, { headRefOid: 'stale' });
  assert.match(f.command(...args, '--pr', PR_URL).stderr, /PR must be published/);
  assert.equal(f.state().slots[0].lease.token, slot.lease.token);
  assert.equal(f.git(f.repo, 'rev-parse', `refs/heads/${BRANCH}`), feature);
});

test('detached confirmation refuses a leased branch that moved', t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  const original = f.git(slot.path, 'rev-parse', 'HEAD');
  f.write(slot.path, 'source', 'branch work to preserve');
  const feature = f.commit(slot.path);
  f.git(slot.path, 'checkout', '--detach', original);

  // Act
  const result = f.command(...f.releaseArgs(slot), '--confirm-detached');

  // Assert: the detached commit is integrated, but the local branch has other work.
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Leased branch differs from verified HEAD/);
  assert.equal(f.git(f.repo, 'rev-parse', `refs/heads/${BRANCH}`), feature);
  assert.equal(f.state().slots[0].lease.token, slot.lease.token);
});

test('publication release rechecks source and branch after verification', t => {
  // Arrange
  const f = fixture(t), slot = f.acquire();
  const args = ['release', '--owner', OWNER, '--token', slot.lease.token, '--pr', PR_URL];
  f.mockPr(slot, {}, `require('node:fs').writeFileSync('late.ts', 'late change');`);

  // Act / Assert
  assert.match(f.command(...args).stderr, /changed during release/);
  assert.equal(f.state().slots[0].lease.token, slot.lease.token);
  rmSync(join(slot.path, 'late.ts'));
  f.mockPr(slot, {}, `require('node:child_process').execFileSync('git', ['checkout', '-b', 'codex/late']);`);
  assert.match(f.command(...args).stderr, /changed during release/);
  assert.equal(f.state().slots[0].lease.token, slot.lease.token);
});

test('killed lock owner retains its lease and wakes the next command', async t => {
  // Arrange: gh signals entry to verification, then blocks on stdin without polling.
  const f = fixture(t), slot = f.acquire();
  const server = await import('node:net');
  const socket = join(f.temp, 'verification.sock');
  const listener = server.createServer();
  t.after(() => listener.close());
  listener.listen(socket);
  await once(listener, 'listening');
  f.mockPr(slot, {}, `const s = require('node:net').connect(${JSON.stringify(socket)}); s.on('connect', () => s.write('ready'));`);
  const connection = once(listener, 'connection');
  const releasing = f.start('release', '--owner', OWNER, '--token', slot.lease.token, '--pr', PR_URL);
  const [client] = await connection;
  await once(client, 'data');
  const status = f.start('status');

  // Act
  releasing.child.kill('SIGKILL');
  await releasing.done;
  const result = await status.done;
  client.end();

  // Assert
  assert.equal(result.status, 0, result.stderr);
  assert.equal(f.state().slots[0].lease.token, slot.lease.token);
});
