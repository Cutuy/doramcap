// Build the systemd_run example unprivileged, then elevate only this TypeScript test executable.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync, watch } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { once } from 'node:events';

assert.equal(process.getuid?.(), 0);
const binary = resolve('target/ci/examples/systemd_run');
const root = mkdtempSync(join(tmpdir(), 'doramcap-systemd-overlays-'));
const quote = JSON.stringify;
const directory = (name: string) => { const path = join(root, name); mkdirSync(path, { recursive: true }); return path; };
const repo = directory('repo');
const source = directory('source');
const readonly = directory('readonly');
writeFileSync(join(readonly, 'read'), 'read-only');
const scope = `/run/doramcap/${createHash('sha256').update(repo).digest('hex').slice(0, 12)}`;
let sequence = 0;

function inputs(plan: string) {
  const id = `run-${++sequence}`;
  const artifacts = join(root, id);
  const runFile = join(root, `${id}.run.pbtxt`);
  const planFile = join(root, `${id}.plan.pbtxt`);
  writeFileSync(runFile, `id:${quote(id)} repository_root:${quote(repo)} artifacts:${quote(artifacts)} readiness_seconds:10 stop_seconds:2 host_shm_access:HOST_SHM_ACCESS_PASSTHROUGH`);
  writeFileSync(planFile, plan);
  return { artifacts, runFile, planFile };
}

function review(input: ReturnType<typeof inputs>) {
  return spawnSync(binary, ['--review', input.runFile, input.planFile], { encoding: 'utf8' });
}

async function waitForFile(path: string) {
  const watcher = watch(resolve(path, '..'));
  try {
    while (!existsSync(path)) await once(watcher, 'change', { signal: AbortSignal.timeout(15000) });
  } finally { watcher.close(); }
}

async function run(input: ReturnType<typeof inputs>, during?: () => Promise<void>) {
  const child = spawn(binary, [input.runFile, input.planFile], { stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '', errors = '';
  const closed = once(child, 'close');
  child.stdout.on('data', chunk => { output += chunk; });
  let ready!: () => void;
  const readiness = new Promise<void>(done => { ready = done; });
  child.stderr.on('data', chunk => { errors += chunk; if (errors.includes('READY')) ready(); });
  const started = await Promise.race([readiness.then(() => true), closed.then(() => false)]);
  if (started) {
    try { if (during) await during(); }
    finally { child.kill('SIGINT'); await closed; }
  }
  await closed;
  assert.ok(output.trim().startsWith('{'), errors);
  return JSON.parse(output);
}

async function isolated(plan: string, file: string) {
  const input = inputs(plan);
  const inspected = review(input);
  assert.equal(inspected.status, 0, inspected.stderr);
  const report = await run(input, async () => {
    const overlays = JSON.parse(readFileSync(join(input.artifacts, 'overlays/prepared.json'), 'utf8'));
    const tree = overlays.trees.find((tree: any) => tree.source === source);
    assert.ok(tree, 'writable directory must have one overlay');
    await waitForFile(join(tree.merged, file));
    assert.equal(existsSync(join(source, file)), false, 'host must remain unchanged during the run');
    assert.equal(readFileSync(join(tree.upper, file), 'utf8').trim(), 'changed');
  });
  assert.equal(report.execution_error, null, JSON.stringify(report));
  assert.equal(report.cleanup_verified, true, JSON.stringify(report));
  assert.equal(readdirSync(scope).length, 0, 'owned runtime resources remain');
  console.log(`PASS: ${file} isolated until stop; runtime cleaned`);
  return { report, input };
}

try {
  // System app: the host working directory exists but is absent in its private root.
  const missing = directory('host-only');
  const invalid = inputs(`programs { id:"missing" argv:"/bin/true" working_directory:${quote(missing)} }`);
  assert.match(review(invalid).stderr, /missing.*working directory.*missing/);
  const rejected = await run(invalid);
  assert.match(rejected.execution_error, /missing.*working directory.*missing/);
  assert.equal(existsSync(join(scope, 'overlays')), false);
  console.log('PASS: missing working directory rejected at Review and Start before mounts');

  await isolated(`programs { id:"created" argv:"/bin/sh" argv:"-ec" argv:"echo changed >/work/created; exec sleep 300" working_directory:"/work" binds { source:${quote(source)} target:"/work" } }`, 'created');

  const own = join(repo, 'own.service');
  writeFileSync(own, `[Service]\nWorkingDirectory=-${root}/absent\nBindPaths=${source}:${source}\nBindReadOnlyPaths=${readonly}:${root}/read\nExecStart=/bin/sh -ec 'test "$(cat ${root}/read/read)" = read-only; echo changed >${source}/own; exec sleep 300'\n`);
  mkdirSync(`${own}.d`);
  writeFileSync(join(`${own}.d`, '20-user.conf'), `[Service]\nBindPaths=-${root}/optional:${root}/optional:norbind\n`);
  const native = `unit_files:${quote(own)}`;
  const inspected = review(inputs(native));
  assert.equal(inspected.status, 0, inspected.stderr);
  assert.match(inspected.stdout, /outside declared writable mounts are not tracked/);
  await isolated(native, 'own');
  console.log('PASS: optional user app working directory and read-only bind preserved; Review declares untracked writes');
} finally {
  const cleanup = spawnSync(binary, ['--cleanup', repo], { encoding: 'utf8' });
  assert.equal(cleanup.status, 0, cleanup.stderr);
  rmSync(root, { recursive: true });
}
