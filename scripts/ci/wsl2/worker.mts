#!/usr/bin/env node
// Bootstrap executes before npm installation; imports must remain dependency-free.
import assert from 'node:assert/strict';
import { chmodSync, closeSync, lstatSync, mkdirSync, openSync, readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PREPARE_PATH } from '../prepare-build.mts';
import { runCommand, SIGNAL_STATUS, WorkerInterrupted } from '../worker-process.mts';

const RUST_TOOLCHAIN = '1.99.1';
const LOCK_CHILD_FD = 3, PRIVATE_DIRECTORY_MODE = 0o700, PRIVATE_FILE_MODE = 0o600;
const NATIVE_LAUNCHER = '/usr/local/libexec/doramcap-ci-native-tests';
class CheckFailed extends Error {
  readonly status: number;
  constructor(program: string, status: number) { super(`${program} failed with exit ${status}`); this.status = status; }
}
/** Existing worker environment overrides; caller environment otherwise passes through unchanged. */
export function nativeWorkerEnvironment(home: string): NodeJS.ProcessEnv {
  return { ...process.env, HOME: home, PATH: `${home}/.local/bin:${home}/.cargo/bin:/usr/local/bin:/usr/bin:/bin`,
    CARGO_TARGET_DIR: join(home, 'target'), CARGO_PROFILE_DEV_DEBUG: 'line-tables-only', CLANG: 'clang-18', RUSTUP_TOOLCHAIN: RUST_TOOLCHAIN };
}
/** Run one native CI job per process, holding the cache lock through unprivileged builds and privileged tests. */
export async function runNativeWorker(snapshot: string, home = homedir()): Promise<number> {
  snapshot = resolve(snapshot);
  const cache = join(home, 'jobs/.build-cache'), source = join(cache, 'source'), target = join(home, 'target');
  const env = nativeWorkerEnvironment(home), controller = new AbortController(), signal = controller.signal;
  const handlers = new Map<keyof typeof SIGNAL_STATUS, () => void>();
  let lockFd: number | undefined;
  const check = async (program: string, args: readonly string[], cwd = source) => {
    const { code } = await runCommand(program, args, { signal, cwd, env, output: 1, lockFd });
    if (code !== 0) throw new CheckFailed(program, code);
  };
  for (const name of Object.keys(SIGNAL_STATUS) as (keyof typeof SIGNAL_STATUS)[]) {
    const handler = () => controller.abort(new WorkerInterrupted(name));
    handlers.set(name, handler); process.on(name, handler);
  }
  try {
    const expected = 'v' + readFileSync(join(snapshot, '.node-version'), 'utf8').trim();
    assert.equal(process.version, expected, `Node version ${process.version}; expected ${expected}`);
    mkdirSync(cache, { recursive: true, mode: PRIVATE_DIRECTORY_MODE });
    const cacheStat = lstatSync(cache);
    assert(cacheStat.isDirectory() && cacheStat.uid === process.getuid!(), `Cache must be an owned, real directory: ${cache}`);
    chmodSync(cache, PRIVATE_DIRECTORY_MODE);
    lockFd = openSync(join(cache, 'worker.lock'), 'a', PRIVATE_FILE_MODE);
    await check('flock', ['--exclusive', String(LOCK_CHILD_FD)], snapshot);
    console.log(`Submitted snapshot: ${snapshot}\nNode: ${process.version}`);
    await check(process.execPath, [join(snapshot, PREPARE_PATH), snapshot, source], snapshot);
    for (const program of ['uname', 'rustc', 'cargo']) await check(program, [program === 'uname' ? '-a' : '--version']);
    console.log(readFileSync(join(source, 'SOURCE.json'), 'utf8'));
    await check('npm', ['--prefix', 'scripts/ci', 'ci', '--ignore-scripts', '--no-audit', '--no-fund']);
    await check('npm', ['--prefix', 'scripts/ci', 'test']);
    await check('npm', ['--prefix', 'scripts/ci', 'run', 'typecheck']);
    await check('rustup', ['component', 'add', 'rustfmt', 'clippy', '--toolchain', RUST_TOOLCHAIN]);
    await check(process.execPath, ['scripts/ci/check-boundaries.mts', '--rules', 'scripts/ci/boundaries.json']);
    await check(process.execPath, ['scripts/ci/test-node.mts']);
    await check(process.execPath, ['scripts/ci/test-mcap.mts']);
    await check('cargo', ['fmt', '--all', '--', '--check']);
    await check('cargo', ['clippy', '--workspace', '--no-deps', '--all-targets', '--locked', '--', '-D', 'warnings']);
    await check('cargo', ['clippy', '--workspace', '--no-deps', '--all-targets', '--all-features', '--locked', '--', '-D', 'warnings']);
    await check('cargo', ['test', '--locked', '-p', 'storage-persistence', '--no-default-features']);
    await check('cargo', ['test', '--locked', '--workspace']);
    await check('cargo', ['test', '--locked', '--workspace', '--all-features']);
    await check('cargo', ['build', '--locked', '--workspace', '--all-features', '--examples', '--bins']);
    await check(join(target, 'debug/examples/policy_overhead'), []);
    await check('rustc', ['--edition=2024', '--crate-name', 'supervisor_fixture', 'libs/workload-supervisor/tests/fixture.inc', '-o', join(target, 'debug/supervisor-test-fixture')]);
    await check('sudo', ['-n', NATIVE_LAUNCHER, source]);
    await check(join(target, 'debug/examples/storage_benchmark'), []);
    return 0;
  } catch (error) {
    console.error((error as Error).stack ?? String(error));
    return error instanceof WorkerInterrupted || error instanceof CheckFailed ? error.status : 1;
  } finally {
    try { if (lockFd !== undefined) closeSync(lockFd); }
    finally { for (const [name, handler] of handlers) process.off(name, handler); }
  }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    assert(process.argv.length === 2, 'Usage: node scripts/ci/wsl2/worker.mts (from a staged job directory)');
    assert(process.platform === 'linux' && process.arch === 'arm64', 'Native worker requires Linux arm64');
    process.exitCode = await runNativeWorker(process.cwd());
  } catch (error) { console.error((error as Error).message); process.exitCode = 1; }
}
