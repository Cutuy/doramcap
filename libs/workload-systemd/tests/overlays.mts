// Build the systemd_run example unprivileged, then elevate only this TypeScript test executable.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { once } from 'node:events';

assert.equal(process.getuid?.(), 0);
const binary = resolve(process.argv[2] ?? 'target/release/examples/systemd_run');
const root = mkdtempSync(join(tmpdir(), 'doramcap-systemd-overlays-'));
const quote = JSON.stringify;
const directory = (name: string) => { const path = join(root, name); mkdirSync(path, { recursive: true }); return path; };
const repo = directory('repo');
const source = directory('source');
const readonly = directory('read-only space:é%n');
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

try {
  // System app: the host working directory exists but is absent in its private root.
  const missing = directory('host-only');
  const invalid = inputs(`programs { id:"missing" argv:"/bin/true" working_directory:${quote(missing)} }`);
  assert.match(review(invalid).stderr, /missing.*working directory.*missing/);
  const rejected = await run(invalid);
  assert.match(rejected.execution_error, /missing.*working directory.*missing/);
  assert.equal(existsSync(join(scope, 'overlays')), false);
  console.log('PASS: missing working directory rejected at Review and Start before mounts');

  const specifier = directory(`spec-doramcap-${scope.split('/').at(-1)}-own.service`);
  writeFileSync(join(specifier, 'read'), 'specifier');
  const own = join(repo, 'own.service');
  writeFileSync(own, `[Service]\nWorkingDirectory=-${root}/absent\nBindPaths=${source}:${source}\nBindReadOnlyPaths=${quote(readonly).replaceAll('%', '%%')}:${root}/read\nBindReadOnlyPaths=${root}/spec-%n:${root}/specifier-read\nExecStart=/bin/sh -ec 'test "$(cat ${root}/read/read)" = read-only; test "$(cat ${root}/specifier-read/read)" = specifier; echo changed >${source}/own; exec sleep 300'\n`);
  mkdirSync(`${own}.d`);
  writeFileSync(join(`${own}.d`, '20-user.conf'), `[Service]\nBindPaths=-${root}/optional:${root}/optional:norbind\n`);
  const native = `unit_files:${quote(own)}`;
  const inspected = review(inputs(native));
  assert.equal(inspected.status, 0, inspected.stderr);
  assert.match(inspected.stdout, /outside declared writable mounts are not tracked/);
  console.log('PASS: optional user app working directory accepted; Review declares untracked writes');

  // Arrange: RootImage paths belong to the image, even when the host path is absent.
  const imageUnit = join(repo, 'camera.service');
  const image = join(root, 'root.img');
  writeFileSync(image, 'image contents are validated by systemd at Start');
  writeFileSync(imageUnit, `[Service]\nRootImage=${image}\nWorkingDirectory=/work/unchecked-camera\nExecStart=/bin/true\n`);
  // Act / Assert
  const imageReview = review(inputs(`unit_files:${quote(imageUnit)}`));
  assert.equal(imageReview.status, 0, imageReview.stderr);
  assert.match(imageReview.stdout, /camera: working directory \/work\/unchecked-camera is inside RootImage and was not checked/);
  console.log('PASS: RootImage Review names the unchecked app and directory');

} finally {
  const cleanup = spawnSync(binary, ['--cleanup', repo], { encoding: 'utf8' });
  assert.equal(cleanup.status, 0, cleanup.stderr);
  rmSync(root, { recursive: true });
}
