// Opt-in host qualification. Build the native addon unprivileged; elevate this script only.
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { randomUUID, createHash } from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { capabilities, execution, launch, systemd } from '../src/native.js';
import { doramcap } from '../src/generated/console.js';

assert.equal(process.geteuid!(), 0, 'Run only this test as root');
const crashRoot = process.argv[2] === '--crash-compose' ? process.argv[3] : null;
const root = crashRoot ?? await mkdtemp(join(tmpdir(), 'real-journal-'));
const project = join(root, 'project');
if (!crashRoot) await mkdir(project);
const owner = await capabilities.open('/var/run/docker.sock', 1);
const { LogIndex, LogState } = doramcap.journal.v1;
const wait = async (predicate: (update: ReturnType<typeof owner.poll>) => boolean, controller = owner) => {
  const deadline = Date.now() + 30000;
  for (;;) {
    const update = controller.poll();
    if (predicate(update)) return update;
    assert.ok(!update.result?.error, update.result?.error ?? '');
    assert.ok(Date.now() < deadline, 'workflow deadline exceeded; trace the surviving process');
    await new Promise(resolve => setTimeout(resolve, 25));
  }
};
async function run(marker: string, fast: boolean, compose = false, failed = false) {
  const id = randomUUID().replaceAll('-', ''), artifacts = join(root, id);
  const request = execution.RunRequest.create({
    run: launch.RunPlan.create({ id, repositoryRoot: project, artifacts, readinessSeconds: 30, stopSeconds: 1, hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH }),
    ...(compose ? { compose: { projects: [{ key: 'app', directory: project, files: ['compose.yaml'] }] } } : {
      systemd: systemd.SystemdPlan.create({ programs: [{ id: 'same-program', argv: failed ? ['/bin/sh', '-c', `printf '${marker}\\n'; exit 7`] : fast ? ['/usr/bin/printf', `${marker}\n`] : ['/bin/sh', '-c', `printf '${marker}\\n'; exec sleep 300`] }] }),
    }),
  });
  const reviewed = await owner.inspect(execution.RunRequest.create({ ...request, run: launch.RunPlan.create({ ...request.run, artifacts: `${artifacts}-review` }) }));
  await owner.start(execution.StartRequest.create({ run: request, ...(compose ? { composeExpected: reviewed.composeSnapshot } : { systemdExpected: reviewed.systemdSnapshot }) }));
  await wait(update => update.ready);
  if (crashRoot) {
    process.send!({ artifacts });
    await new Promise(() => {}); // The parent deliberately crashes this ready controller.
  }
  await owner.stop(id);
  const finished = await wait(update => !!update.result);
  assert.equal(finished.result!.boundaryClean, true);
  assert.equal(finished.result!.error, null);
  const index = LogIndex.decode(await readFile(join(artifacts, 'logs/index.pb')));
  const texts = await Promise.all(index.programs.map((program, number) => program.state === LogState.LOG_STATE_EXPORTED ? readFile(join(artifacts, `logs/${number}.log`), 'utf8') : Promise.resolve('')));
  return { index, texts, finished };
}
try {
  if (crashRoot) await run('compose-journal', false, true);
  else {
    // Arrange / Act: the same unit name runs twice with distinct stdout markers.
    const first = await run('journal-first', false), second = await run('journal-second', false);
    // Assert
    assert.notEqual(first.index.programs[0].identity, second.index.programs[0].identity);
    assert.ok(first.texts[0].includes('journal-first')); assert.ok(!first.texts[0].includes('journal-second'));
    assert.ok(second.texts[0].includes('journal-second')); assert.ok(!second.texts[0].includes('journal-first'));
    let missingIdentity = 0, missingOutput = 0;
    for (let n = 0; n < 5; n++) {
      const marker = `instant-${n}`, result = await run(marker, true);
      if (!result.index.programs[0].identity) missingIdentity++;
      if (!result.texts.join('').includes(marker)) missingOutput++;
    }
    console.log(`systemd: adjacent invocation isolation passed; instant-exit sample: ${missingIdentity}/5 missing identities, ${missingOutput}/5 missing stdout markers.`);
    const failed = await run('failed-program', false, false, true);
    assert.ok(failed.finished.result!.serviceExits?.some(exit => exit.exitCode === 7));
    assert.ok(failed.texts.join('').includes('failed-program'));
    // Stop an adopted service while its notify startup job is still pending.
    const cancelling = randomUUID().replaceAll('-', ''), cancellingArtifacts = join(root, cancelling);
    const unit = join(project, 'cancelled.service');
    await writeFile(unit, '[Service]\nType=notify\nExecStart=/bin/sh -c "echo cancelled-program; exec sleep 300"\n');
    const request = execution.RunRequest.create({ run: { id: cancelling, repositoryRoot: project, artifacts: cancellingArtifacts, readinessSeconds: 30, stopSeconds: 1, hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH }, systemd: { unitFiles: [unit] } });
    const reviewed = await owner.inspect(request);
    await owner.start(execution.StartRequest.create({ run: request, systemdExpected: reviewed.systemdSnapshot }));
    const name = `doramcap-${createHash('sha256').update(project).digest('hex').slice(0, 12)}-cancelled.service`;
    const deadline = Date.now() + 10000;
    while (Number(execFileSync('systemctl', ['show', '--property=MainPID', '--value', name], { encoding: 'utf8' })) === 0) {
      assert.ok(Date.now() < deadline, 'startup process absent; leave it alive for tracing');
      await new Promise(resolve => setTimeout(resolve, 25));
    }
    await owner.stop(cancelling);
    assert.equal((await wait(update => !!update.result)).result!.boundaryClean, true);
    const cancelled = LogIndex.decode(await readFile(join(cancellingArtifacts, 'logs/index.pb')));
    assert.equal(cancelled.programs[0].state, LogState.LOG_STATE_EXPORTED);
    console.log('systemd: program failure and cancellation during startup both exported.');
    // Project-managed logging remains explicitly skipped alongside collected containers.
    await writeFile(join(project, 'compose.yaml'), `services:\n  collected:\n    image: busybox:latest\n    command: [sh, -c, "echo compose-journal; exec sleep 300"]\n  managed:\n    image: busybox:latest\n    command: [sleep, '300']\n    logging:\n      driver: local\n`);
    const composed = await run('compose-journal', false, true);
    assert.ok(composed.index.programs.some(program => program.state === LogState.LOG_STATE_SKIPPED));
    const collected = composed.index.programs.findIndex(program => program.state === LogState.LOG_STATE_EXPORTED);
    assert.ok(collected >= 0); assert.equal(composed.index.programs[collected].identity!.length, 64);
    assert.ok(composed.texts[collected].includes('compose-journal'));
    console.log('Compose: full container identity, journal export and project logging skip passed.');
    // Arrange: another Console backend keeps a different repository running during reset.
    const otherProject = join(root, 'other-project'), otherArtifacts = join(root, 'other-run');
    await mkdir(otherProject);
    await writeFile(join(otherProject, 'compose.yaml'), `services:\n  other:\n    image: busybox:latest\n    command: [sh, -c, "echo other-repository; exec sleep 300"]\n`);
    const otherOwner = await capabilities.open('/var/run/docker.sock', 1);
    const otherId = randomUUID().replaceAll('-', '');
    const otherRequest = execution.RunRequest.create({
      run: { id: otherId, repositoryRoot: otherProject, artifacts: otherArtifacts, readinessSeconds: 30, stopSeconds: 1, hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH },
      compose: { projects: [{ key: 'app', directory: otherProject, files: ['compose.yaml'] }] },
    });
    let otherStarted = false;
    try {
      const otherReview = await otherOwner.inspect(execution.RunRequest.create({ ...otherRequest,
        run: launch.RunPlan.create({ ...otherRequest.run, artifacts: `${otherArtifacts}-review` }) }));
      await otherOwner.start(execution.StartRequest.create({ run: otherRequest, composeExpected: otherReview.composeSnapshot }));
      otherStarted = true;
      await wait(update => !!update.ready, otherOwner);
      const otherIds = execFileSync('docker', ['ps', '--no-trunc', '--filter', `label=io.doramcap.artifacts=${otherArtifacts}`, '--format', '{{.ID}}'], { encoding: 'utf8' }).trim().split('\n').filter(Boolean);
      assert.ok(otherIds.length > 0, 'the independent repository has live containers');
      const child = spawn(process.execPath, ['--import', 'tsx', process.argv[1], '--crash-compose', root], { stdio: ['ignore', 'ignore', 'pipe', 'ipc'] });
      let diagnostic = '';
      child.stderr!.on('data', bytes => { diagnostic = (diagnostic + bytes).slice(-8192); });
      try {
        const ready = once(child, 'message');
        const earlyExit = once(child, 'exit').then(() => { throw new Error(`crash fixture exited before readiness: ${diagnostic}`); });
        const [{ artifacts: abandoned }] = await Promise.race([ready, earlyExit]) as [{ artifacts: string }];
        const original = LogIndex.decode(await readFile(join(abandoned, 'logs/index.pb')));
        const ids = original.programs.map(program => program.identity).sort();
        assert.ok(ids.length > 0);
        const exit = once(child, 'exit');
        child.kill('SIGKILL');
        await exit;
        // Act: remove only fixture metadata to model a crash before identity persistence.
        await rm(join(abandoned, 'logs/index.pb'));
        await run('compose-journal', false, true);
        const recovered = LogIndex.decode(await readFile(join(abandoned, 'logs/index.pb')));
        // Assert: labels alone reconstruct the prior IDs and frozen export/skip choices.
        assert.deepEqual(recovered.programs.map(program => program.identity).sort(), ids);
        assert.ok(recovered.programs.some(program => program.state === LogState.LOG_STATE_EXPORTED));
        assert.ok(recovered.programs.some(program => program.state === LogState.LOG_STATE_SKIPPED));
        assert.ok(!recovered.programs.some(program => program.state === LogState.LOG_STATE_PENDING));
        for (const id of otherIds) {
          assert.equal(execFileSync('docker', ['inspect', '--format', '{{.State.Running}}', id], { encoding: 'utf8' }).trim(), 'true');
        }
        console.log('Compose: reset reconstructed a missing index from container labels after a controller crash; the other repository kept running.');
      } finally {
        if (child.exitCode === null && child.signalCode === null) {
          const exit = once(child, 'exit');
          child.kill('SIGKILL');
          await exit;
        }
      }
    } finally {
      try {
        if (otherStarted) {
          await otherOwner.stop(otherId);
          const finished = await wait(update => !!update.result, otherOwner);
          assert.equal(finished.result!.boundaryClean, true);
        }
      } finally { await otherOwner.close(); }
    }
  }
} finally {
  await owner.close();
  if (!crashRoot) await rm(root, { recursive: true, force: true });
}
