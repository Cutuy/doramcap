// Build the systemd_run example unprivileged, then elevate only this TypeScript test executable.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync, watch } from 'node:fs';
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
const readonly = directory('read-only space:é%n');
writeFileSync(join(readonly, 'read'), 'read-only');
const scope = `/run/doramcap/${createHash('sha256').update(repo).digest('hex').slice(0, 12)}`;
let sequence = 0;

function inputs(plan: string, policy = "WRITE_POLICY_WRITE_BACK") {
  const id = `run-${++sequence}`;
  const artifacts = join(root, id);
  const runFile = join(root, `${id}.run.pbtxt`);
  const planFile = join(root, `${id}.plan.pbtxt`);
  writeFileSync(runFile, `id:${quote(id)} repository_root:${quote(repo)} artifacts:${quote(artifacts)} readiness_seconds:10 stop_seconds:2 host_shm_access:HOST_SHM_ACCESS_PASSTHROUGH write_policy:${policy}`);
  writeFileSync(planFile, plan);
  return { artifacts, runFile, planFile };
}

function review(input: ReturnType<typeof inputs>) {
  return spawnSync(binary, ['--review', input.runFile, input.planFile], { encoding: 'utf8' });
}

async function waitForFile(path: string) {
  const watcher = watch(resolve(path, '..'));
  try {
    while (!existsSync(path) || statSync(path).size === 0) await once(watcher, 'change', { signal: AbortSignal.timeout(15000) });
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
  assert.equal(report.filesystem_results.unmounted, true);
  assert.ok(report.filesystem_changes.host_binds.counts.added > 0);
  assert.equal(readFileSync(join(source, file), 'utf8').trim(), 'changed', 'write-back must happen after stop');
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
  await isolated(native, 'own');
  console.log('PASS: optional user app working directory and read-only bind preserved; Review declares untracked writes');

  const writer = (file: string) => `programs { id:"writer" argv:"/bin/sh" argv:"-ec" argv:${quote(`echo changed >/work/${file}; exec sleep 300`)} binds { source:${quote(source)} target:"/work" } }`;
  const discard = inputs(writer('discarded'), 'WRITE_POLICY_DISCARD');
  const discarded = await run(discard, async () => {
    const overlay = JSON.parse(readFileSync(join(discard.artifacts, 'overlays/prepared.json'), 'utf8')).trees[0];
    await waitForFile(join(overlay.merged, 'discarded'));
  });
  assert.equal(discarded.filesystem_results.disposition, 'discard');
  assert.equal(discarded.cleanup_verified, true);
  assert.equal(existsSync(join(source, 'discarded')), false);
  console.log('PASS: discard preserves the host and still publishes file changes');

  for (const policy of ['WRITE_POLICY_KEEP', 'WRITE_POLICY_DISCARD']) {
    for (const kind of ['system app', 'user app']) {
      const file = `${policy}-${kind}`;
      writeFileSync(join(source, 'existing'), 'before');
      writeFileSync(join(source, 'removed'), 'remove');
      const target = kind === 'user app' ? source : '/work';
      const script = `echo modified >${target}/existing; rm ${target}/removed; echo changed >${target}/${file}; exec sleep 300`;
      const unit = join(repo, 'policy.service');
      writeFileSync(unit, `[Service]\n${kind === 'user app' ? `BindPaths=${source}:${source}\n` : ''}ExecStart=/bin/sh -ec '${script}'\n`);
      const plan = kind === 'system app'
        ? `programs { id:"policy" argv:"/bin/sh" argv:"-ec" argv:${quote(script)} binds { source:${quote(source)} target:"/work" } }`
        : `unit_files:${quote(unit)}`;
      const input = inputs(plan, policy);
      assert.equal(review(input).status, 0);
      const report = await run(input, async () => {
        const overlay = JSON.parse(readFileSync(join(input.artifacts, 'overlays/prepared.json'), 'utf8')).trees[0];
        await waitForFile(join(overlay.merged, file));
        assert.equal(existsSync(join(source, file)), false);
      });
      assert.equal(report.execution_error, null, JSON.stringify(report));
      assert.equal(report.cleanup_verified, true, JSON.stringify(report));
      assert.equal(report.filesystem_results.disposition, policy === 'WRITE_POLICY_KEEP' ? 'keep' : 'discard');
      assert.equal(readFileSync(join(source, 'existing'), 'utf8'), 'before');
      assert.equal(readFileSync(join(source, 'removed'), 'utf8'), 'remove');
      assert.equal(existsSync(join(source, file)), false);
      assert.ok(report.filesystem_changes.host_binds.counts.deleted > 0);
      if (policy === 'WRITE_POLICY_KEEP') {
        const kept = report.filesystem_results.paths[0];
        assert.equal(readFileSync(join(kept, file), 'utf8').trim(), 'changed');
        assert.equal(readFileSync(join(kept, 'existing'), 'utf8').trim(), 'modified');
        assert.equal(existsSync(join(kept, 'removed')), false);
        assert.equal(existsSync(join(scope, 'overlays')), false);
      } else assert.equal(existsSync(join(input.artifacts, 'kept')), false);
      console.log(`PASS: ${kind} ${policy} preserves the host and reports added, modified and deleted files`);
    }
  }

  const copyFailure = inputs(writer('copy-failure'), 'WRITE_POLICY_KEEP');
  let failedUpper = '';
  const copyFailed = await run(copyFailure, async () => {
    const overlay = JSON.parse(readFileSync(join(copyFailure.artifacts, 'overlays/prepared.json'), 'utf8')).trees[0];
    failedUpper = overlay.upper;
    await waitForFile(join(overlay.merged, 'copy-failure'));
    writeFileSync(join(copyFailure.artifacts, 'kept'), 'prevent destination creation');
  });
  assert.equal(copyFailed.cleanup_verified, false);
  assert.equal(copyFailed.filesystem_results, null);
  assert.ok(copyFailed.filesystem_changes);
  assert.equal(readFileSync(join(failedUpper, 'copy-failure'), 'utf8').trim(), 'changed');
  assert.equal(existsSync(join(source, 'copy-failure')), false);
  assert.equal(spawnSync(binary, ['--cleanup', repo]).status, 0);
  console.log('PASS: keep-copy failure retains the upper and blocks cleanup until reset');

  const failed = inputs(`programs { id:"failure" argv:"/does-not-exist" binds { source:${quote(source)} target:"/work" } }`);
  const rollback = await run(failed);
  assert.ok(rollback.execution_error);
  assert.equal(rollback.cleanup_verified, true, JSON.stringify(rollback));
  console.log('PASS: launch failure rolls back owned overlays and units');

  const saveFailure = inputs(writer('unsaved'));
  const unsaved = await run(saveFailure, async () => {
    const overlay = JSON.parse(readFileSync(join(saveFailure.artifacts, 'overlays/prepared.json'), 'utf8')).trees[0];
    await waitForFile(join(overlay.merged, 'unsaved'));
    mkdirSync(join(saveFailure.artifacts, 'filesystem/report.pb'));
  });
  assert.equal(unsaved.filesystem_changes, null);
  assert.equal(unsaved.filesystem_results, null);
  assert.equal(unsaved.cleanup_verified, false);
  assert.equal(existsSync(join(source, 'unsaved')), false);
  assert.equal(spawnSync(binary, ['--cleanup', repo]).status, 0);
  console.log('PASS: report-save failure preserves uppers; reset discards them');

  let holder: ReturnType<typeof spawn> | undefined;
  const busy = inputs(writer('busy'));
  try {
    const report = await run(busy, async () => {
      const overlay = JSON.parse(readFileSync(join(busy.artifacts, 'overlays/prepared.json'), 'utf8')).trees[0];
      await waitForFile(join(overlay.merged, 'busy'));
      holder = spawn(process.execPath, ['-e', 'process.stdout.write("READY"); process.stdin.resume()'], { cwd: overlay.merged, stdio: ['pipe', 'pipe', 'pipe'] });
      await once(holder.stdout!, 'data');
    });
    assert.equal(report.filesystem_results, null);
    assert.equal(report.cleanup_verified, false);
    assert.match(report.diagnostics.join(' '), /busy/i);
    assert.equal(existsSync(join(source, 'busy')), false);
  } finally {
    if (holder) { const closed = once(holder, 'close'); holder.stdin!.end(); await closed; }
    assert.equal(spawnSync(binary, ['--cleanup', repo]).status, 0);
  }
  console.log('PASS: a busy overlay blocks removal; cleanup succeeds after its holder exits');

  const lingering = join(repo, 'lingering.service');
  writeFileSync(lingering, `[Service]\nKillMode=process\nWorkingDirectory=${source}\nBindPaths=${source}:${source}\nExecStart=/bin/sh -ec 'sleep 300 & echo $$! >${source}/child.pid; exec sleep 300'\n`);
  let childPid = 0;
  try {
    const input = inputs(`unit_files:${quote(lingering)}`);
    const report = await run(input, async () => {
      const overlay = JSON.parse(readFileSync(join(input.artifacts, 'overlays/prepared.json'), 'utf8')).trees[0];
      await waitForFile(join(overlay.merged, 'child.pid'));
      childPid = Number(readFileSync(join(overlay.merged, 'child.pid'), 'utf8').trim());
      assert.ok(childPid > 0);
    });
    assert.equal(report.writers_stopped, false);
    assert.equal(report.filesystem_changes, null);
    assert.equal(report.cleanup_verified, false);
    assert.match(report.diagnostics.join(' '), /processes remain/);
  } finally {
    if (childPid) process.kill(childPid, 'SIGTERM');
    const deadline = Date.now() + 5000;
    while (childPid && existsSync(`/proc/${childPid}`) && Date.now() < deadline) await new Promise(done => setTimeout(done, 10));
    assert.equal(spawnSync(binary, ['--cleanup', repo]).status, 0);
  }
  console.log('PASS: inactive units with live descendants cannot publish changes or release overlays');

  // Deliberate controller crash after readiness, followed by the next admission's explicit reset.
  const interrupted = inputs(writer('interrupted'));
  const controller = spawn(binary, [interrupted.runFile, interrupted.planFile], { stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  await new Promise<void>(done => controller.stderr.on('data', chunk => { stderr += chunk; if (stderr.includes('READY')) done(); }));
  const overlay = JSON.parse(readFileSync(join(interrupted.artifacts, 'overlays/prepared.json'), 'utf8')).trees[0];
  await waitForFile(join(overlay.merged, 'interrupted'));
  const crashed = once(controller, 'close');
  controller.kill('SIGKILL');
  await crashed;
  assert.equal(existsSync(join(source, 'interrupted')), false);
  assert.equal(spawnSync(binary, ['--cleanup', repo]).status, 0);
  assert.equal(existsSync(join(source, 'interrupted')), false);
  assert.equal(existsSync(join(scope, 'overlays')), false);
  console.log('PASS: controller restart stops old writers before discarding leftover overlays');

} finally {
  const cleanup = spawnSync(binary, ['--cleanup', repo], { encoding: 'utf8' });
  assert.equal(cleanup.status, 0, cleanup.stderr);
  rmSync(root, { recursive: true });
}
