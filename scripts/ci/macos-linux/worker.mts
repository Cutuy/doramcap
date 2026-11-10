#!/usr/bin/env node
import assert from 'node:assert/strict';
import { accessSync, chmodSync, closeSync, constants, copyFileSync, lstatSync, mkdirSync, openSync, readFileSync, renameSync, rmSync, writeFileSync, writeSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { hashFile } from '../hash-file.mts';
import { verifyArtifacts, writeArtifactManifest } from './artifacts.mts';
import { installNextest } from './install-nextest.mts';
import { runCommand, SIGNAL_STATUS, WorkerInterrupted, type WorkerCommand } from '../worker-process.mts';

const CACHE_DIRECTORY = '/tmp/doramcap-ci-run-only';
const PRIVATE_DIRECTORY_MODE = 0o700, PRIVATE_FILE_MODE = 0o600, EXECUTABLE_MODE = 0o755;
const PRIVATE_UMASK = 0o077;
const LOCK_CHILD_FD = 3, TEST_THREADS = 1;
const COMPOSE_PLUGIN = '/usr/libexec/docker/cli-plugins/docker-compose';
const SUITES = [
  { name: 'default', options: ['--workspace'] },
  { name: 'all-features', options: ['--workspace', '--all-features'] },
  { name: 'storage-no-default', options: ['-p', 'storage-persistence', '--no-default-features'] },
] as const;
export type WorkerMode = 'build' | 'run';
export type WorkerOptions = { mode: WorkerMode; job: string; cache?: string; home?: string; composePlugin?: string };
/** Validate the CLI platform separately from job execution, which fixtures exercise on either Linux architecture. */
export function validateWorkerPlatform(platform = process.platform, arch = process.arch): void {
  assert(platform === 'linux' && arch === 'arm64', 'Worker requires Linux arm64');
}
function installExecutable(source: string, target: string): void {
  mkdirSync(dirname(target), { recursive: true });
  const temporary = target + '.installing';
  try { copyFileSync(source, temporary); chmodSync(temporary, EXECUTABLE_MODE); renameSync(temporary, target); }
  finally { rmSync(temporary, { force: true }); }
}
function executablePath(name: string, path: string): string {
  for (const directory of path.split(':')) {
    const candidate = join(directory, name);
    try { accessSync(candidate, constants.X_OK); return candidate; }
    catch (error) { if (!['ENOENT', 'EACCES'].includes((error as NodeJS.ErrnoException).code ?? '')) throw error; }
  }
  throw new Error(`Missing build tool: ${name}`);
}
/** Execute a staged job with exclusive cache ownership; signals retain nonzero status after child cleanup. */
export async function runWorker(options: WorkerOptions): Promise<number> {
  const job = resolve(options.job), cache = resolve(options.cache ?? CACHE_DIRECTORY), home = options.home ?? homedir();
  const artifacts = join(job, 'artifacts'), source = join(job, 'source'), cachedSource = join(cache, 'source');
  const bin = join(home, '.local/bin'), target = join(cache, 'target');
  const controller = new AbortController(), signal = controller.signal;
  const handlers = new Map<keyof typeof SIGNAL_STATUS, () => void>();
  let result = 1, lockFd: number | undefined, output = 1;
  const log = (message: string) => { writeSync(output, message + '\n'); };
  const env: NodeJS.ProcessEnv = { ...process.env, HOME: home, PATH: `${bin}:${join(home, '.cargo/bin')}:/usr/local/bin:/usr/bin:/bin` };
  const command: WorkerCommand = (program, args, settings = {}) => runCommand(program, args, {
    signal, cwd: settings.cwd ?? job, env, output, lockFd, capture: settings.capture,
  });
  const checked: WorkerCommand = async (program, args, settings) => {
    const completed = await command(program, args, settings);
    assert.equal(completed.code, 0, `${program} failed with exit ${completed.code}`);
    return completed;
  };
  const previousUmask = process.umask(PRIVATE_UMASK);
  for (const name of Object.keys(SIGNAL_STATUS) as (keyof typeof SIGNAL_STATUS)[]) {
    const handler = () => controller.abort(new WorkerInterrupted(name));
    handlers.set(name, handler); process.on(name, handler);
  }
  try {
    if (options.mode === 'run') {
      rmSync(join(job, 'exit-code'), { force: true }); rmSync(join(job, 'results.tsv'), { force: true });
      output = openSync(join(job, 'run.log'), 'w', PRIVATE_FILE_MODE);
    }
    signal.throwIfAborted();
    const expectedNode = 'v' + readFileSync(join(source, '.node-version'), 'utf8').trim();
    assert.equal(process.version, expectedNode, `Node version ${process.version}; expected ${expectedNode}`);
    mkdirSync(cache, { recursive: true, mode: PRIVATE_DIRECTORY_MODE });
    const cacheStat = lstatSync(cache);
    assert(cacheStat.isDirectory() && cacheStat.uid === process.getuid!(), `Cache must be an owned, real directory: ${cache}`);
    chmodSync(cache, PRIVATE_DIRECTORY_MODE);
    lockFd = openSync(join(cache, 'worker.lock'), 'a', PRIVATE_FILE_MODE);
    // flock shares this open file description; children inherit it to protect active cache consumers.
    await checked('flock', ['--exclusive', String(LOCK_CHILD_FD)]);
    log(`Node: ${process.version}`);
    await checked('uname', ['-a']);
    if (options.mode === 'build') {
      const compose = options.composePlugin ?? COMPOSE_PLUGIN;
      assert(lstatSync(compose, { throwIfNoEntry: false })?.isFile(), `Missing Docker Compose plugin: ${compose}`);
      for (const program of ['rustc', 'cargo', 'docker']) await checked(program, ['--version']);
      await checked('docker', ['compose', 'version']);
      await checked(process.execPath, [join(source, 'scripts/ci/prepare-build.mts'), source, cachedSource]);
      env.CARGO_TARGET_DIR = target;
      env.CARGO_PROFILE_DEV_DEBUG = 'line-tables-only';
      env.CLANG = 'clang-18';
      const inSource: WorkerCommand = (program, args, settings) => command(program, args, { ...settings, cwd: cachedSource });
      await installNextest(bin, inSource, signal);
      mkdirSync(artifacts, { recursive: true });
      for (const suite of SUITES) await checked('cargo', ['nextest', 'archive', '--locked', ...suite.options, '--archive-file', join(artifacts, `${suite.name}.tar.zst`)], { cwd: cachedSource });
      copyFileSync(join(source, 'source.tar.gz'), join(artifacts, 'source.tar.gz'));
      copyFileSync(join(bin, 'cargo-nextest'), join(artifacts, 'cargo-nextest'));
      const docker = executablePath('docker', env.PATH!);
      copyFileSync(docker, join(artifacts, 'docker')); copyFileSync(compose, join(artifacts, 'docker-compose'));
      await writeArtifactManifest(artifacts, signal);
      result = 0;
    } else {
      await verifyArtifacts(artifacts, signal);
      rmSync(cachedSource, { recursive: true, force: true }); rmSync(target, { recursive: true, force: true });
      mkdirSync(cachedSource);
      await checked('tar', ['-xzpf', join(artifacts, 'source.tar.gz'), '-C', cachedSource]);
      installExecutable(join(artifacts, 'cargo-nextest'), join(bin, 'cargo-nextest'));
      installExecutable(join(artifacts, 'docker'), join(bin, 'docker'));
      installExecutable(join(artifacts, 'docker-compose'), join(home, '.docker/cli-plugins/docker-compose'));
      await checked('cargo-nextest', ['--version']); await checked('docker', ['--version']); await checked('docker', ['compose', 'version']);
      log(readFileSync('/proc/self/limits', 'utf8').split('\n').find(line => line.startsWith('Max locked memory')) ?? 'Memlock: unavailable');
      log(`Source manifest SHA256: ${await hashFile(join(cachedSource, 'SOURCE.json'), signal)}`);
      for (const name of ['memory.max', 'memory.events']) {
        try { log(`${name}: ${readFileSync(join('/sys/fs/cgroup', name), 'utf8').trim()}`); }
        catch { log(`${name}: unavailable`); }
      }
      writeFileSync(join(job, 'results.tsv'), '', { mode: PRIVATE_FILE_MODE }); result = 0;
      for (const suite of SUITES) {
        rmSync(target, { recursive: true, force: true }); log(`Suite: ${suite.name}`);
        const completed = await command('cargo-nextest', ['nextest', 'run', '--archive-file', join(artifacts, `${suite.name}.tar.zst`),
          '--extract-to', cache, '--test-threads', String(TEST_THREADS), '--no-fail-fast'], { cwd: cachedSource });
        writeFileSync(join(job, 'results.tsv'), `${suite.name}\t${completed.code}\n`, { flag: 'a' });
        if (completed.code !== 0) result = 1;
      }
      log('Privileged native tests and doctests remain on the WSL worker.');
    }
  } catch (error) {
    result = error instanceof WorkerInterrupted ? error.status : 1;
    log((error as Error).stack ?? String(error));
  } finally {
    try {
      if (lockFd !== undefined) closeSync(lockFd);
      if (signal.aborted && signal.reason instanceof WorkerInterrupted) result = signal.reason.status;
      if (options.mode === 'run') {
        const temporary = join(job, '.exit-code.tmp');
        writeFileSync(temporary, `${result}\n`, { mode: PRIVATE_FILE_MODE }); renameSync(temporary, join(job, 'exit-code'));
      }
    } finally {
      for (const [name, handler] of handlers) process.off(name, handler);
      if (output !== 1) closeSync(output);
      process.umask(previousUmask);
    }
  }
  return result;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 3 || !['build', 'run'].includes(process.argv[2])) {
      console.error('Usage: macos-linux/worker.mts build|run (from a staged job directory)'); process.exitCode = 2;
    } else {
      validateWorkerPlatform();
      process.exitCode = await runWorker({ mode: process.argv[2] as WorkerMode, job: process.cwd() });
    }
  } catch (error) { console.error((error as Error).message); process.exitCode = 1; }
}
