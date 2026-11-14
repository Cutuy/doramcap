// Build the systemd_run example unprivileged, then elevate only this TypeScript test executable.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync, watch, chmodSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { once } from 'node:events';

assert.equal(process.getuid?.(), 0);
const binary = resolve(process.argv[2] ?? 'target/release/examples/systemd_run');
const root = mkdtempSync(join(tmpdir(), 'doramcap-systemd-overlays-'));
const quote = JSON.stringify;
const statusCalls = join(root, 'systemctl-status-called');
const statusTools = join(root, 'status-tools');
mkdirSync(statusTools);
const systemctl = join(statusTools, 'systemctl');
writeFileSync(systemctl, `#!${process.execPath}\nrequire('node:fs').writeFileSync(${quote(statusCalls)}, process.argv.slice(2).join(' ')); process.exit(1);\n`);
chmodSync(systemctl, 0o755);
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

async function run(input: ReturnType<typeof inputs>, during?: () => Promise<void>, stopOnReady = true) {
  const child = spawn(binary, [input.runFile, input.planFile], { env: { ...process.env, PATH: `${statusTools}:${process.env.PATH}` }, stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '', errors = '';
  const closed = once(child, 'close');
  child.stdout.on('data', chunk => { output += chunk; });
  let ready!: () => void;
  const readiness = new Promise<void>(done => { ready = done; });
  child.stderr.on('data', chunk => { errors += chunk; if (errors.includes('READY')) ready(); });
  const started = await Promise.race([readiness.then(() => true), closed.then(() => false)]);
  if (started && stopOnReady) {
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

  // Arrange: systemd resolves WorkingDirectory=~ to the User= home at Start.
  const homeUnit = join(repo, 'home.service');
  writeFileSync(homeUnit, `[Service]\nWorkingDirectory=~\nExecStart=/bin/true\n`);
  // Act / Assert
  const homeReview = review(inputs(`unit_files:${quote(homeUnit)}`));
  assert.equal(homeReview.status, 0, homeReview.stderr);
  assert.match(homeReview.stdout, /home: working directory ~ is the User= home and was not checked/);
  console.log('PASS: WorkingDirectory=~ is noted at Review, not rejected');

  // Arrange: a native pre-start command writes through the mounted overlay, then removes
  // its working directory. The following ExecStart must fail with systemd's CHDIR reason.
  for (const type of ['exec', 'simple']) for (const discardWrites of [false, true]) {
    const working = directory(`late-directory-${type}-${discardWrites}`);
    const unit = join(repo, 'late.service');
    const input = inputs(`unit_files:${quote(unit)}`, discardWrites ? 'WRITE_POLICY_DISCARD' : 'WRITE_POLICY_WRITE_BACK');
    const file = `late-${type}-${discardWrites}`;
    writeFileSync(unit, `[Service]\nType=${type}\nWorkingDirectory=${working}\nBindPaths=${source}:/work\nExecStartPre=/bin/sh -ec 'echo changed >/work/${file}; echo output >${input.artifacts}/native-output; rmdir ${working}'\nExecStart=/bin/true\n`);
    assert.equal(review(input).status, 0);
    // Act
    const report = await run(input, undefined, false);
    // Assert: the native reason survives into both the returned and stored run result.
    assert.equal(existsSync(statusCalls), false, 'startup diagnostics must use D-Bus without systemctl');
    const stored = JSON.parse(readFileSync(join(input.artifacts, 'report.json'), 'utf8'));
    assert.equal(stored.execution_error, report.execution_error);
    assert.deepEqual(stored.unit_failures, report.unit_failures);
    assert.equal(report.unit_failures.length, 1);
    const failure = report.unit_failures[0];
    assert.match(failure.unit, /late\.service$/);
    assert.ok(failure.cause);
    assert.equal(report.execution_error, failure.cause);
    assert.match(report.execution_error, /result=exit-code, status=200/);
    assert.equal(failure.service.result, 'exit-code');
    assert.equal(failure.service.exec_main_code, 1);
    assert.equal(failure.service.exec_main_status, 200);
    assert.equal(report.writers_stopped, true, JSON.stringify(report));
    assert.equal(report.cleanup_verified, true, JSON.stringify(report));
    assert.ok(report.units.some((unit: any) => unit.exec_main_status === 200));
    assert.equal(report.filesystem_results.disposition, discardWrites ? 'discard' : 'write_back');
    assert.equal(report.filesystem_results.unmounted, true);
    assert.ok(report.filesystem_changes.host_binds.counts.added > 0);
    assert.equal(existsSync(join(source, file)), !discardWrites);
    assert.equal(readFileSync(join(input.artifacts, 'native-output'), 'utf8').trim(), 'output');
    assert.equal(readdirSync(scope).length, 0);
    for (const unit of report.units) {
      assert.notEqual(spawnSync('systemctl', ['is-active', unit.unit], { encoding: 'utf8' }).stdout.trim(), 'active');
    }
  }
  console.log('PASS: late native CHDIR failure preserves the reason, stops units and honors both write policies');

  // Arrange: hold the first rsync operation on a FIFO so Stop lands during partial setup.
  // PATH selects a test fixture; application configuration is unchanged.
  const gate = join(root, 'setup-gate');
  assert.equal(spawnSync('mkfifo', [gate]).status, 0);
  const marker = join(root, 'setup-entered');
  const tools = directory('tools');
  const rsync = join(tools, 'rsync');
  writeFileSync(rsync, `#!${process.execPath}\nconst fs = require('node:fs'); const cp = require('node:child_process'); fs.writeFileSync(${quote(marker)}, 'entered'); fs.readFileSync(${quote(gate)}); process.exit(cp.spawnSync('/usr/bin/rsync', process.argv.slice(2), {stdio:'inherit'}).status ?? 1);\n`);
  chmodSync(rsync, 0o755);
  for (const dropCaller of [false, true]) {
    rmSync(marker, { force: true });
    const cancelled = inputs(native);
    writeFileSync(cancelled.runFile, readFileSync(cancelled.runFile, 'utf8').replace('HOST_SHM_ACCESS_PASSTHROUGH', 'HOST_SHM_ACCESS_BLOCKED'));
    const setup = spawn(binary, [...(dropCaller ? ['--drop-on-stop'] : []), cancelled.runFile, cancelled.planFile], { env: { ...process.env, PATH: `${tools}:${process.env.PATH}` }, stdio: ['ignore', 'pipe', 'pipe'] });
    const setupClosed = once(setup, 'close');
    let setupOutput = '', setupErrors = '';
    let stopped!: () => void;
    const stopObserved = new Promise<void>(done => { stopped = done; });
    setup.stdout.on('data', chunk => { setupOutput += chunk; });
    setup.stderr.on('data', chunk => { setupErrors += chunk; if (setupErrors.includes(dropCaller ? 'CLEANUP' : 'STOP')) stopped(); });
    await waitForFile(marker);
    assert.equal(existsSync(join(scope, 'units')), true);
    assert.equal(existsSync(join(scope, 'overlays/0/upper')), true);
    // Act: Stop must drain the in-flight operation, suppress further setup and clean its partial resources.
    setup.kill('SIGINT');
    await stopObserved;
    assert.equal(setup.exitCode, null, 'cleanup must wait for the in-flight setup operation');
    assert.equal(spawnSync('flock', ['--exclusive', '--nonblock', scope, '/bin/true']).status, 1, 'in-flight setup must retain its scope lock even after its caller is dropped');
    writeFileSync(gate, 'continue');
    await setupClosed;
    // Assert
    const cancelledReport = JSON.parse(setupOutput);
    assert.equal(cancelledReport.cleanup_verified, true, JSON.stringify(cancelledReport));
    if (!dropCaller) {
      assert.equal(cancelledReport.execution_error, null, setupErrors);
      assert.deepEqual(cancelledReport.units, []);
    }
    assert.doesNotMatch(setupErrors, /READY/);
    assert.equal(existsSync(join(cancelled.artifacts, 'overlays/prepared.json')), false);
    assert.equal(readdirSync(scope).length, 0);
    assert.equal(spawnSync(binary, ['--cleanup', repo]).status, 0);
    assert.equal(spawnSync(binary, ['--cleanup', repo]).status, 0);
    assert.equal(readdirSync(scope).length, 0);
  }
  console.log('PASS: Stop and caller cancellation drain partial setup; Cleanup is repeatable');

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
    for (const kind of ['system', 'user']) {
      const file = `${policy}-${kind}`;
      writeFileSync(join(source, 'existing'), 'before');
      writeFileSync(join(source, 'removed'), 'remove');
      const target = kind === 'user' ? source : '/work';
      const script = `echo modified >${target}/existing; rm ${target}/removed; echo changed >${target}/${file}; exec sleep 300`;
      const unit = join(repo, 'policy.service');
      writeFileSync(unit, `[Service]\n${kind === 'user' ? `BindPaths=${source}:${source}\n` : ''}ExecStart=/bin/sh -ec '${script}'\n`);
      const plan = kind === 'system'
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
      console.log(`PASS: ${kind} app ${policy} preserves the host and reports added, modified and deleted files`);
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
