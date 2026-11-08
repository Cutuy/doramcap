import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { execFileSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { join, resolve, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { ConsoleService } from '../src/service.js';
import { capabilities, execution, launch, systemd, wire } from '../src/native.js';
import { stageSystemAppInputs } from '../../scripts/system-app-inputs.mjs';

/** How long the run records after it reports running. */
const RECORDING_MS = 5000;
/** Upper bound for start, stop and removal together. */
const RUN_DEADLINE_MS = 90000;
/** Status polling interval; this opt-in test observes Console like the browser does. */
const POLL_MS = 100;

const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');

test('systemd Start rejects changed inputs and handles immediate Stop', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-systemd-admission-'));
  const project = join(root, 'project'), runs = join(root, 'runs');
  await mkdir(project);
  await mkdir(runs);
  const unit = join(project, 'workload.service');
  const original = '[Service]\nExecStart=/bin/sleep 300\n';
  await writeFile(unit, original);
  const owner = await capabilities.open(join(root, 'absent-docker.sock'), 1);
  const run = (id: string) => execution.RunRequest.create({
    run: launch.RunPlan.create({
      id, repositoryRoot: project, artifacts: join(runs, id),
      readinessSeconds: 5, stopSeconds: 1,
    }),
    systemd: systemd.SystemdPlan.create({ unitFiles: [unit] }),
  });
  const reviewed = await owner.inspect(run('review'));
  try {
    // Act / Assert: changed bytes reject before launch and publish terminal facts first.
    await writeFile(unit, '[Service]\nExecStart=/bin/sleep 301\n');
    await assert.rejects(owner.start(execution.StartRequest.create({
      run: run('changed'), systemdExpected: reviewed.systemdSnapshot,
    })), /ConfigurationChanged/);
    assert.equal(owner.poll().result!.error, 'ConfigurationChanged');
    assert.equal(owner.poll().result!.blocked, false);
    await assert.rejects(stat(join(runs, 'changed')), { code: 'ENOENT' });

    // Act / Assert: immediate Stop either rejects pending admission or cleans up an admitted run.
    await writeFile(unit, original);
    const started = owner.start(execution.StartRequest.create({
      run: run('stopped'), systemdExpected: reviewed.systemdSnapshot,
    }));
    const admission = started.then(() => true, (error: Error) => {
      assert.match(error.message, /Stopped/);
      return false;
    });
    await owner.stop('stopped');
    const accepted = await admission;
    const deadline = Date.now() + RUN_DEADLINE_MS;
    let stopped = owner.poll();
    while (!stopped.result && Date.now() < deadline) {
      await new Promise((done) => setTimeout(done, POLL_MS));
      stopped = owner.poll();
    }
    assert.ok(stopped.result, 'Stop must publish completion');
    assert.equal(stopped.result.blocked, false);
    if (accepted) assert.equal(stopped.result.boundaryClean, true);
    else {
      assert.equal(stopped.result.error, 'Stopped');
      await assert.rejects(stat(join(runs, 'stopped')), { code: 'ENOENT' });
    }

    // Act / Assert: unchanged inputs admit under a fresh run ID and artifact directory.
    await owner.start(execution.StartRequest.create({
      run: run('unchanged'), systemdExpected: reviewed.systemdSnapshot,
    }));
    await owner.close();
    const finished = owner.poll();
    assert.equal(finished.runId, 'unchanged');
    assert.equal(finished.result!.boundaryClean, true);
    assert.equal(finished.result!.error, null);
  } finally {
    await owner.close();
    await rm(root, { recursive: true, force: true });
  }
});

/**
 * Opt-in systemd test: build binaries unprivileged, put `dora`, `system-monitor` and
 * `fault-watchdog` in one directory named by DORAMCAP_TEST_BINARIES, and run only this
 * test executable as root.
 */
test('Console runs CPU monitoring, Fault Watchdog and a user unit under systemd', async () => {
  // Arrange
  const binaries = process.env.DORAMCAP_TEST_BINARIES;
  assert.ok(binaries, 'DORAMCAP_TEST_BINARIES names the host binaries directory');
  const root = await mkdtemp(join(tmpdir(), 'console-systemd-'));
  const project = join(root, 'project'), apps = join(root, 'system-apps');
  await mkdir(project);
  await writeFile(join(project, 'workload.service'), '[Service]\nExecStart=/bin/sleep 300\n');
  await cp(join(repository, 'console/release/system-apps'), apps, { recursive: true });
  stageSystemAppInputs(repository, apps);
  await writeFile(join(root, 'secret'), 'test-secret', { mode: 0o600 });
  const config = join(root, 'console.pbtxt');
  await writeFile(
    config,
    `form: SYSTEMD binaries:"${binaries}" listen_address:"127.0.0.1:8080" public_origin:"http://127.0.0.1:8080" ` +
      `project_root:"${project}" data_root:"${root}/data" docker_socket:"${root}/absent-docker.sock" access_secret_file:"${root}/secret" ` +
      `runtime { readiness_seconds:30 stop_seconds:5 } ` +
      `dora_apps:"dora-coordinator" dora_apps:"dora-daemon" ` +
      `apps { manifest:"${apps}/dora-coordinator/dora-coordinator.pbtxt" } ` +
      `apps { manifest:"${apps}/dora-daemon/dora-daemon.pbtxt" } ` +
      `apps { manifest:"${apps}/cpu-monitor/cpu-monitor.pbtxt" } ` +
      `apps { manifest:"${apps}/fault-watchdog/fault-watchdog.pbtxt" }`,
  );
  const { backend } = await ConsoleService.open(config);
  const status = async () => wire.Status.decode(await backend.status());
  const wait = async (phases: number[]) => {
    const deadline = Date.now() + RUN_DEADLINE_MS;
    let current = await status();
    while (phases.includes(current.phase) && Date.now() < deadline) {
      await new Promise((done) => setTimeout(done, POLL_MS));
      current = await status();
    }
    return current;
  };
  try {
    const review = wire.Review.decode(
      await backend.prepare(Buffer.from(wire.PrepareRequest.encode({
        profiles: [],
        systemApps: { ids: ['cpu-monitor', 'fault-watchdog'], profiles: { 'cpu-monitor': 'record', 'fault-watchdog': 'mcap' } },
      }).finish())),
    );
    // Act
    await backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'systemd-run' }).finish()));
    const running = await wait([wire.Phase.STARTING]);
    // Assert
    assert.equal(running.phase, wire.Phase.RUNNING, JSON.stringify(wire.Status.toObject(running, { enums: String })));
    // Act
    await new Promise((done) => setTimeout(done, RECORDING_MS));
    await backend.stop(Buffer.from(wire.StopRequest.encode({ runId: review.runId }).finish()));
    const finished = await wait([wire.Phase.RUNNING, wire.Phase.STOPPING]);
    // Assert
    const outcome = JSON.stringify(wire.Status.toObject(finished, { enums: String }));
    assert.equal(finished.phase, wire.Phase.FINISHED, outcome);
    assert.equal(finished.outcome!.boundaryClean, true, outcome);
    assert.equal(finished.outcome!.recordingFinalized, true, outcome);
    for (const name of ['cpu.mcap', 'faults.mcap'])
      assert.ok((await stat(join(root, 'data/runs', review.runId, name))).size > 0, name);
    const units = execFileSync('systemctl', ['list-units', '--all', '--plain', '--no-legend', 'doramcap-*'], { encoding: 'utf8' });
    assert.equal(units.trim(), '', units);
  } finally {
    await backend.close();
    // Console records each failure's cause here; show it before the directory goes.
    const diagnostics = join(root, 'data/diagnostics');
    for (const name of await readdir(diagnostics).catch(() => []))
      console.error(await readFile(join(diagnostics, name), 'utf8'));
    await rm(root, { recursive: true, force: true });
  }
});
