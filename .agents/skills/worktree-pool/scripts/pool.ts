#!/usr/bin/env node
/** Persistent worktree leases; Linux, Node 22.18+, Git and util-linux flock. */
import { spawnSync } from 'node:child_process';
import {
  closeSync, existsSync, lstatSync, mkdirSync, openSync, readFileSync, readdirSync,
  readlinkSync, realpathSync, renameSync, rmSync, symlinkSync, writeFileSync,
} from 'node:fs';
import { randomUUID } from 'node:crypto';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { parseArgs } from 'node:util';

const SLOT_COUNT = 5;
const LOCK_FD = 3;
const MAX_COMMAND_OUTPUT = 64 * 1024 * 1024;
const PRIVATE_FILE_MODE = 0o600;
const FILE_MODE_MASK = 0o777;
const BUILD_OUTPUTS = [
  'target', 'frontend/dist', 'apps/*/frontend/dist', 'apps/*/frontend/generated',
  'apps/*/frontend/src/generated', 'apps/*/frontend/src/.generated',
  'apps/*/frontend/.vite', 'apps/*/frontend/.vite-temp',
  'apps/*/frontend/test-results', 'apps/*/frontend/playwright-report',
  'apps/project-explorer/frontend/public/content', 'apps/cuda-shm-demo/build',
  'console/server/build', 'console/server/dist', 'console/server/src/generated',
];
type Lease = { token: string; owner: string; branch: string; base: string; state: 'preparing' | 'active' };
type Slot = { id: number; path: string; lease: Lease | null; [key: string]: unknown };
type State = { slots: Slot[] };
type SourceFile = { path: string; data: Buffer; link: boolean; mode: number };

function run(command: string, args: string[], cwd?: string, input?: Buffer): Buffer {
  const result = spawnSync(command, args, { cwd, input, maxBuffer: MAX_COMMAND_OUTPUT });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command}: ${result.stderr?.toString().trim() || result.signal || result.status}`);
  return result.stdout;
}
const git = (root: string, ...args: string[]) => run('git', ['-C', root, ...args]);
const dirt = (root: string) => git(root, 'status', '--short', '--untracked-files=all').toString();
const head = (root: string) => git(root, 'rev-parse', 'HEAD').toString().trim();
const branch = (root: string) => git(root, 'branch', '--show-current').toString().trim();
const print = (value: unknown) => console.log(JSON.stringify(value, null, 2));

function stat(path: string) {
  try { return lstatSync(path); }
  catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return undefined;
    throw error;
  }
}

/** Validate every output before deleting any; preserve source, caches and local data. */
function cleanBuildOutputs(root: string) {
  const apps = stat(join(root, 'apps')) ? readdirSync(join(root, 'apps')) : [];
  const candidates = BUILD_OUTPUTS.flatMap(pattern => pattern.includes('*')
    ? apps.map(app => pattern.replace('*', app)) : [pattern]);
  const outputs: string[] = [];
  for (const rel of new Set(candidates)) {
    let current = root;
    let present = true;
    for (const part of rel.split('/')) {
      current = join(current, part);
      const info = stat(current);
      if (!info) { present = false; break; }
      if (info.isSymbolicLink()) throw new Error(`Build output uses a symlink; inspect manually: ${current}`);
      if (!info.isDirectory()) {
        if (current === join(root, rel)) throw new Error(`Build output is not a directory: ${current}`);
        present = false;
        break;
      }
    }
    if (!present) continue;
    if (git(root, 'ls-files', '--', rel).length) throw new Error(`Build output contains tracked files: ${join(root, rel)}`);
    outputs.push(join(root, rel));
  }
  for (const output of outputs) {
    console.error(`Removing build output: ${output}`);
    rmSync(output, { recursive: true });
  }
}

function snapshot(root: string) {
  if (git(root, 'ls-files', '-u').length) throw new Error('Main checkout has unresolved conflicts');
  const files: SourceFile[] = git(root, 'ls-files', '--others', '--exclude-standard', '-z')
    .toString().split('\0').filter(Boolean).map(path => {
      const source = join(root, path);
      const info = lstatSync(source);
      return { path, link: info.isSymbolicLink(), mode: info.mode & FILE_MODE_MASK,
        data: info.isSymbolicLink() ? Buffer.from(readlinkSync(source)) : readFileSync(source) };
    });
  return { head: head(root), patch: git(root, 'diff', '--binary', '--ignore-submodules=all', 'HEAD', '--'), files };
}

function publicState(state: State) {
  // Tokens are returned only to the acquiring owner, including failed preparation.
  return { slots: state.slots.map(slot => ({ id: slot.id, path: slot.path,
    lease: slot.lease ? { owner: slot.lease.owner, branch: slot.lease.branch,
      base: slot.lease.base, state: slot.lease.state } : null })) };
}

function main() {
  const { values, positionals } = parseArgs({ allowPositionals: true, options: {
    repo: { type: 'string', default: '.' }, owner: { type: 'string' },
    branch: { type: 'string' }, token: { type: 'string' }, pr: { type: 'string' },
    main: { type: 'boolean' }, 'discard-dirty': { type: 'boolean' }, help: { type: 'boolean' },
  } });
  if (values.help) {
    console.log(`Usage: node pool.ts [--repo CHECKOUT] COMMAND
  init
  status
  acq --owner TASK --branch codex/NAME [--token TOKEN] [--discard-dirty]
  release --owner TASK --token TOKEN (--pr URL | --main)

Five non-expiring leases. Save the acquisition token. Resume with the same owner,
branch and token. Commands block on the shared flock; a full pool fails explicitly.
--discard-dirty requires prior user approval for the displayed path and files.`);
    return;
  }
  const command = positionals[0] === 'acquire' ? 'acq' : positionals[0];
  const allowed: Record<string, string[]> = {
    init: [], status: [], acq: ['owner', 'branch', 'token', 'discard-dirty'],
    release: ['owner', 'token', 'pr', 'main'],
  };
  if (positionals.length !== 1 || !Object.hasOwn(allowed, command)) throw new Error('Expected init, status, acq or release; see --help');
  for (const key of Object.keys(values)) {
    if (key !== 'repo' && !allowed[command].includes(key)) throw new Error(`Unexpected --${key} for ${command}`);
  }
  if (['acq', 'release'].includes(command) && !values.owner?.trim()) throw new Error('--owner is required');
  if (command === 'acq' && !values.branch?.trim()) throw new Error('--branch is required');
  if (command === 'release' && (!values.token || Boolean(values.main) === Boolean(values.pr))) {
    throw new Error('Release requires --token and exactly one of --pr or --main');
  }
  const common = realpathSync(git(values.repo, 'rev-parse', '--path-format=absolute', '--git-common-dir').toString().trim());
  const mainRoot = realpathSync(git(values.repo, 'worktree', 'list', '--porcelain', '-z').toString().split('\0')[0].slice('worktree '.length));
  if (git(mainRoot, 'rev-parse', '--is-bare-repository').toString().trim() === 'true') throw new Error('A non-bare main checkout is required');
  const base = join(common, 'agent-pool');
  mkdirSync(base, { recursive: true });
  const lock = openSync(join(base, 'lock'), 'a', PRIVATE_FILE_MODE);
  try {
    // flock and Node share this open file description; Node holds it until close.
    const locked = spawnSync('flock', ['--exclusive', String(LOCK_FD)], { stdio: ['ignore', 'pipe', 'pipe', lock] });
    if (locked.error) throw locked.error;
    if (locked.status !== 0) throw new Error(`Cannot lock pool: ${locked.stderr?.toString() || locked.signal}`);
    const stateFile = join(base, 'state.json');
    const state: State = existsSync(stateFile) ? JSON.parse(readFileSync(stateFile, 'utf8')) : { slots: [] };
    if (!Array.isArray(state.slots) || new Set(state.slots.map(s => s.id)).size !== state.slots.length) throw new Error('Invalid pool state');
    const save = () => {
      const temporary = join(base, 'state.tmp');
      writeFileSync(temporary, JSON.stringify(state, null, 2) + '\n', { mode: PRIVATE_FILE_MODE });
      renameSync(temporary, stateFile);
    };
    const safe = (slot: Slot) => {
      const expected = join(base, 'worktrees', String(slot.id));
      if (!Number.isInteger(slot.id) || slot.id < 1 || slot.id > SLOT_COUNT || slot.path !== expected || slot.path === mainRoot) throw new Error('Unsafe pool path; refusing mutation');
      for (const path of [base, join(base, 'worktrees'), expected]) {
        if (stat(path)?.isSymbolicLink()) throw new Error(`Unsafe symlink in pool path: ${path}`);
      }
      if (existsSync(expected) && (realpathSync(git(expected, 'rev-parse', '--show-toplevel').toString().trim()) !== expected
        || realpathSync(git(expected, 'rev-parse', '--path-format=absolute', '--git-common-dir').toString().trim()) !== common)) {
        throw new Error('Pool directory is not a worktree of this repository');
      }
      return expected;
    };
    const owned = () => {
      const slot = state.slots.find(s => s.lease?.token === values.token);
      if (!slot || slot.lease!.owner !== values.owner) throw new Error('Lease owner and token do not match');
      const path = safe(slot);
      if (slot.lease!.state !== 'active') throw new Error('Incomplete acquisition needs manual repair; lease remains reserved');
      if (branch(path) !== slot.lease!.branch) throw new Error('Worktree branch differs from leased branch');
      return slot;
    };
    if (command === 'init') {
      for (let id = 1; id <= SLOT_COUNT; id++) {
        if (state.slots.some(s => s.id === id)) continue;
        const slot: Slot = { id, path: join(base, 'worktrees', String(id)), lease: null };
        const path = safe(slot);
        if (existsSync(path)) throw new Error(`Unregistered path ${path}; inspect manually`);
        git(mainRoot, 'worktree', 'add', '--detach', path, 'HEAD');
        state.slots.push(slot);
        save();
      }
    } else if (command === 'acq') {
      if (values.token) {
        const slot = owned();
        if (slot.lease!.branch !== values.branch) throw new Error('Resume requires the leased branch');
        print(slot);
        return;
      }
      if (state.slots.length !== SLOT_COUNT) throw new Error(`Run init first; pool must contain ${SLOT_COUNT} slots`);
      if (state.slots.some(s => s.lease?.owner === values.owner)) throw new Error('Owner already has a lease; resume with its saved --token');
      git(mainRoot, 'check-ref-format', '--branch', values.branch!);
      const ref = spawnSync('git', ['-C', mainRoot, 'show-ref', '--verify', '--quiet', `refs/heads/${values.branch}`]);
      if (ref.status !== 1) throw new Error('Branch already exists or could not be checked; use a new branch');
      const free = state.slots.filter(s => s.lease === null);
      if (!free.length) throw new Error('Pool exhausted; leases never expire. Wait for an explicit release.');
      let slot = free[0];
      for (const candidate of free) {
        cleanBuildOutputs(safe(candidate));
        if (!dirt(candidate.path)) { slot = candidate; break; }
      }
      const path = safe(slot);
      const dirty = dirt(path);
      if (dirty) {
        console.error(`WARNING: refreshing ${path} will discard tracked, untracked and ignored files:\n${git(path, 'status', '--short', '--ignored', '--untracked-files=all')}`);
        if (!values['discard-dirty']) throw new Error('Cleanup blocked. Obtain user approval before --discard-dirty.');
      }
      const source = snapshot(mainRoot);
      slot.lease = { owner: values.owner!, token: randomUUID(), branch: values.branch!, base: source.head, state: 'preparing' };
      save();
      try {
        if (dirty) {
          git(path, 'reset', '--hard', 'HEAD');
          git(path, 'clean', '-fdx');
          if (dirt(path)) throw new Error('Cleanup left source files; inspect manually');
        }
        git(path, 'checkout', '--no-overwrite-ignore', '--detach', source.head);
        git(path, 'checkout', '-b', values.branch!);
        if (source.patch.length) run('git', ['-C', path, 'apply', '--binary', '-'], undefined, source.patch);
        for (const file of source.files) {
          const destination = resolve(path, file.path);
          const rel = relative(path, destination);
          if (!rel || rel === '..' || rel.startsWith(`..${sep}`) || isAbsolute(rel)) throw new Error(`Unsafe destination: ${file.path}`);
          let parent = path;
          for (const part of relative(path, dirname(destination)).split(sep).filter(Boolean)) {
            parent = join(parent, part);
            if (stat(parent)?.isSymbolicLink()) throw new Error(`Unsafe destination: ${file.path}`);
            mkdirSync(parent, { recursive: true });
          }
          if (stat(destination)) throw new Error(`Existing slot file conflicts with main snapshot: ${file.path}`);
          if (file.link) symlinkSync(file.data.toString(), destination);
          else writeFileSync(destination, file.data, { flag: 'wx', mode: file.mode });
        }
        cleanBuildOutputs(path);
        if (JSON.stringify(snapshot(mainRoot)) !== JSON.stringify(source)) throw new Error('Main checkout changed during refresh; inspect the reserved slot');
        slot.lease.state = 'active';
        save();
      } catch (error) {
        console.error(`Refresh failed; slot remains reserved for ${values.owner} with token ${slot.lease.token}. Inspect and repair manually; never steal a lease.`);
        throw error;
      }
      print(slot);
      return;
    } else if (command === 'release') {
      const slot = owned();
      const path = slot.path;
      const lease = slot.lease!;
      const featureHead = head(path);
      cleanBuildOutputs(path);
      if (dirt(path)) throw new Error(`Worktree has uncommitted changes; commit and integrate them before release:\n${dirt(path)}`);
      if (values.main) {
        const mainHead = git(mainRoot, 'rev-parse', 'refs/heads/main').toString().trim();
        if (git(path, 'cherry', mainHead, featureHead).toString().split('\n').some(line => line.startsWith('+'))) throw new Error('Feature patches remain unintegrated into main');
        slot.last_local_integration = { branch: lease.branch, feature_head: featureHead, main_head: mainHead,
          authorization: 'User-requested local cherry-pick integration and release' };
      } else {
        const pr = JSON.parse(run('gh', ['pr', 'view', values.pr!, '--json', 'url,state,headRefName,headRefOid'], path).toString());
        if (pr.url !== values.pr || !['OPEN', 'MERGED'].includes(pr.state) || pr.headRefName !== lease.branch || pr.headRefOid !== featureHead) throw new Error('PR must be published for the leased branch at its current HEAD');
        slot.last_pr = pr.url;
      }
      cleanBuildOutputs(path);
      if (dirt(path) || head(path) !== featureHead || branch(path) !== lease.branch) throw new Error('Worktree changed during release; lease remains active');
      slot.lease = null;
      save();
    }
    print(publicState(state));
  } finally {
    closeSync(lock);
  }
}

try { main(); }
catch (error) { console.error(`Error: ${(error as Error).message}`); process.exitCode = 1; }
