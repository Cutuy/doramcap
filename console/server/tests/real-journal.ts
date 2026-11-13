// Opt-in host qualification. Build the native addon unprivileged; elevate this script only.
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { capabilities, execution, launch, systemd } from '../src/native.js';
import { doramcap } from '../src/generated/console.js';

assert.equal(process.geteuid!(), 0, 'Run only this test as root');
const root = await mkdtemp(join(tmpdir(), 'real-journal-'));
const project = join(root, 'project');
await mkdir(project);
const owner = await capabilities.open('/var/run/docker.sock', 1);
const { LogIndex, LogState } = doramcap.journal.v1;
const wait = async (predicate: (update: ReturnType<typeof owner.poll>) => boolean) => {
  const deadline = Date.now() + 30000;
  for (;;) {
    const update = owner.poll();
    if (predicate(update)) return update;
    assert.ok(!update.result?.error, update.result?.error ?? '');
    assert.ok(Date.now() < deadline, 'workflow deadline exceeded; trace the surviving process');
    await new Promise(resolve => setTimeout(resolve, 25));
  }
};
async function run(marker: string, fast: boolean, compose = false) {
  const id = randomUUID().replaceAll('-', ''), artifacts = join(root, id);
  const request = execution.RunRequest.create({
    run: launch.RunPlan.create({ id, repositoryRoot: project, artifacts, readinessSeconds: 30, stopSeconds: 1, hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH }),
    ...(compose ? { compose: { projects: [{ key: 'app', directory: project, files: ['compose.yaml'] }] } } : {
      systemd: systemd.SystemdPlan.create({ programs: [{ id: 'same-program', argv: fast ? ['/usr/bin/printf', `${marker}\n`] : ['/bin/sh', '-c', `printf '${marker}\\n'; exec sleep 300`] }] }),
    }),
  });
  const reviewed = await owner.inspect(execution.RunRequest.create({ ...request, run: launch.RunPlan.create({ ...request.run, artifacts: `${artifacts}-review` }) }));
  await owner.start(execution.StartRequest.create({ run: request, ...(compose ? { composeExpected: reviewed.composeSnapshot } : { systemdExpected: reviewed.systemdSnapshot }) }));
  await wait(update => update.ready);
  await owner.stop(id);
  const finished = await wait(update => !!update.result);
  assert.equal(finished.result!.boundaryClean, true);
  assert.equal(finished.result!.error, null);
  const index = LogIndex.decode(await readFile(join(artifacts, 'logs/index.pb')));
  const texts = await Promise.all(index.programs.map((program, number) => program.state === LogState.LOG_STATE_EXPORTED ? readFile(join(artifacts, `logs/${number}.log`), 'utf8') : Promise.resolve('')));
  return { index, texts };
}
try {
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
  // Project-managed logging remains explicitly skipped alongside collected containers.
  await writeFile(join(project, 'compose.yaml'), `services:\n  collected:\n    image: busybox:latest\n    command: [sh, -c, "echo compose-journal; exec sleep 300"]\n  managed:\n    image: busybox:latest\n    command: [sleep, '300']\n    logging:\n      driver: local\n`);
  const composed = await run('compose-journal', false, true);
  assert.ok(composed.index.programs.some(program => program.state === LogState.LOG_STATE_SKIPPED));
  const collected = composed.index.programs.findIndex(program => program.state === LogState.LOG_STATE_EXPORTED);
  assert.ok(collected >= 0); assert.equal(composed.index.programs[collected].identity!.length, 64);
  assert.ok(composed.texts[collected].includes('compose-journal'));
  console.log('Compose: full container identity, journal export and project logging skip passed.');
} finally {
  await owner.close();
  await rm(root, { recursive: true, force: true });
}
