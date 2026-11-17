import { strict as assert } from 'node:assert';
import { createHash, randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { access, mkdir, readFile, writeFile } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { setTimeout as pause } from 'node:timers/promises';
import { capabilities, execution, holoscan, launch } from '../../../console/server/src/native.js';
import { doramcap } from '../../../console/server/src/generated/console.js';
import { sourceState } from '../../../console/server/src/records/source.js';
import { exportFileChanges } from '../../../console/server/src/records/file-changes.js';

const [cli, source, store, output, scenario = 'complete', application = 'first_pipeline', language = 'python', mode = 'finite', suppliedImage] = process.argv.slice(2);
assert.ok([cli, source, store, output].every(path => path && isAbsolute(path)),
  'usage: tsx application-real.mts /cli /source /builds /output [complete|stop|stale|keep|write-discard|cli-fail|cancel-before-cid|cli-killed|wrong-image|wrong-placement|recover] [app] [language] [mode] [wrong-image sha256:ID]');
assert.ok(['complete', 'stop', 'stale', 'keep', 'write-discard', 'cli-fail', 'cancel-before-cid', 'cli-killed', 'wrong-image', 'wrong-placement', 'recover'].includes(scenario));
const exec = promisify(execFile);
const POLL_MS = 20, CASE_MS = 180_000;
await mkdir(output, { recursive: true, mode: 0o700 });
await mkdir(join(output, 'runs'), { mode: 0o700 }).catch(error => { if (error.code !== 'EEXIST') throw error; });
const data = join(output, "data $literal 'quoted'\nline");
await mkdir(data, { mode: 0o755 }).catch(error => { if (error.code !== 'EEXIST') throw error; });
const initial = await sourceState(source);
assert.equal(initial.sourceState, launch.SourceState.SOURCE_STATE_CLEAN, 'retained source fixture is clean');
const id = `m5-${scenario}-${randomUUID()}`;
const artifacts = join(output, 'runs', id);
const injected = ['cli-fail', 'cancel-before-cid'].includes(scenario);
const invalidContainer = ['wrong-image', 'wrong-placement'].includes(scenario);
const wrapped = injected || invalidContainer || scenario === 'cli-killed';
const selectedCli = wrapped ? join(output, `${scenario}.mjs`) : cli;
const childMarker = join(output, `${id}-cli.pid`);
const owner = await capabilities.open('/var/run/docker.sock', 1);
try {
  const selection = { application, language, mode };
  const build = await owner.buildLookup(holoscan.BuildStoreRequest.create({ root: store, key: { repositoryRoot: source, selection } }));
  assert.ok(build.record?.imageId, build.diagnostics.join('; '));
  const alternateImage = suppliedImage;
  if (scenario === 'wrong-image') {
    assert.match(alternateImage ?? '', /^sha256:[a-f0-9]{64}$/, 'supply the immutable ID of a known cached test image');
    assert.notEqual(alternateImage, build.record!.imageId);
    assert.equal((await exec('docker', ['image', 'inspect', '--format', '{{.Id}}', alternateImage!])).stdout.trim(), alternateImage, 'explicit test image already exists; no pull or fallback');
  }
  const alternateParent = scenario === 'wrong-placement'
    ? ((await exec('docker', ['info', '--format', '{{.CgroupDriver}}'])).stdout.trim() === 'systemd' ? 'system.slice' : '/') : '';
  // Qualification-only mutations; native queries/routing and the frozen source/image stay unchanged.
  if (wrapped) await writeFile(selectedCli, `#!/usr/bin/node
  import { writeFileSync } from 'node:fs';
  let args = process.argv.slice(2);
  if (args.includes('run') && !args.includes('--dryrun')) {
    writeFileSync(${JSON.stringify(childMarker)}, String(process.pid));
    ${scenario === 'cli-fail' ? 'process.stderr.write("injected native CLI failure before cidfile\\n"); process.exit(7);'
      : scenario === 'cancel-before-cid' ? 'setTimeout(() => process.exit(8), 300000);'
      : `
    ${scenario === 'wrong-image' ? `args.splice(args.indexOf('--'), 0, '--img=${alternateImage}');` : ''}
    ${scenario === 'wrong-placement' ? `args = args.map(arg => arg.startsWith('--docker-opts=') ? arg + ' --cgroup-parent=${alternateParent}' : arg);` : ''}
    process.execve(${JSON.stringify(cli)}, [${JSON.stringify(cli)}, ...args], process.env);`}
  } else { process.execve(${JSON.stringify(cli)}, [${JSON.stringify(cli)}, ...args], process.env); }
  `, { mode: 0o755 });
  const request = execution.RunRequest.create({
    run: { id, repositoryRoot: source, artifacts, readinessSeconds: 30, stopSeconds: 5,
      hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH,
      writePolicy: scenario === 'keep' ? doramcap.filesystem.v1.WritePolicy.WRITE_POLICY_KEEP : doramcap.filesystem.v1.WritePolicy.WRITE_POLICY_DISCARD },
    holoscan: { plan: { cli: selectedCli, selection, extraDataFolders: [data] }, currentSource: initial, buildStoreRoot: store },
  });
  const review = holoscan.Preparation.fromObject((await owner.inspect(request)).holoscan!);
  assert.deepEqual(review.rejections, [], review.rejections.join('; '));
  assert.ok(review.snapshot?.sha256);
  if (scenario === 'stale') {
    review.snapshot!.sha256 = 'stale-review';
    await assert.rejects(owner.start(execution.StartRequest.create({ run: request, holoscanExpected: review.snapshot! })), /Review|changed/i);
    await assert.rejects(access(artifacts), { code: 'ENOENT' });
    assert.deepEqual(await sourceState(source), initial);
    console.log(JSON.stringify({ scenario, source: initial, rejectedBeforeArtifacts: true }));
    process.exitCode = 0;
  } else {
    await owner.start(execution.StartRequest.create({ run: request, holoscanExpected: review.snapshot! }));
    console.log(JSON.stringify({ phase: 'admitted', scenario, id }));
    const deadline = Date.now() + CASE_MS;
    let stopping = false, wrote = false, killed = false;
    let update: ReturnType<typeof owner.poll>;
    for (;;) {
      update = owner.poll();
      assert.equal(update.runId, id);
      if (update.result) break;
      if (scenario === 'cancel-before-cid' && !stopping) {
        try { await access(childMarker); stopping = true; await owner.stop(id); }
        catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error; }
      }
      if (scenario === 'cli-killed' && update.ready && !killed) {
        const cid = (await readFile(join(artifacts, 'holoscan-container.cid'), 'utf8')).trim();
        const state = async () => JSON.parse((await exec('docker', ['inspect', '--format', '{{json .State}}', cid])).stdout);
        assert.equal((await state()).Running, true);
        process.kill(Number(await readFile(childMarker, 'utf8')), 'SIGKILL');
        assert.equal((await state()).Running, true, 'native application survives actual CLI death');
        killed = true;
      }
      if (['keep', 'write-discard'].includes(scenario) && update.ready && !wrote) {
        const cid = (await readFile(join(artifacts, 'holoscan-container.cid'), 'utf8')).trim();
        const info = JSON.parse((await exec('docker', ['inspect', cid])).stdout)[0];
        const project = info.Mounts.find((mount: { Source: string }) => mount.Source.endsWith('/merged/source'));
        assert.ok(project, 'actual controlled project mount exists');
        const paths = [project.Destination + '/m5-controlled.txt', `/workspace/volumes/${data.split('/').at(-1)}/m5-controlled.txt`];
        await exec('docker', ['exec', cid, 'python3', '-c', 'import sys; from pathlib import Path; [Path(p).write_text("m5 controlled write") for p in sys.argv[1:]]', ...paths]);
        wrote = true;
      }
      if (scenario === 'stop' && update.ready && !stopping) {
        const cid = (await readFile(join(artifacts, 'holoscan-container.cid'), 'utf8')).trim();
        const state = JSON.parse((await exec('docker', ['inspect', '--format', '{{json .State}}', cid])).stdout);
        assert.equal(state.Running, true, 'Stop qualification requires an actually running native container');
        stopping = true;
        await owner.stop(id);
      }
      assert.ok(Date.now() < deadline, 'native application case deadline expired');
      await pause(POLL_MS);
    }
    if (scenario === 'stop') assert.ok(stopping, 'case did not exercise stopping a live application');
    const result = update.result!;
    assert.ok(result.holoscanReport, result.error ?? 'typed Holoscan report is present');
    const report = holoscan.HoloscanRunReport.fromObject(result.holoscanReport);
    await writeFile(join(output, `${scenario}-result.json`), JSON.stringify(result, null, 2), { mode: 0o600 });
    console.log(JSON.stringify({ phase: 'terminal', scenario, id, error: result.error, artifacts, report: report.toJSON() }));
    if (injected) {
      if (scenario === 'cli-fail') {
        assert.match(report.executionError, /CLI exited before an owned application/);
        assert.equal(report.cliExitCode, 7);
        assert.match(await readFile(join(artifacts, report.stderr), 'utf8'), /injected native CLI failure/);
      } else {
        assert.ok(stopping, 'cancel before cidfile was actually signaled');
        assert.equal(report.executionError, '');
        assert.equal(report.cliSignal, 9);
      }
      assert.equal(report.stopped?.writersStopped, false, 'unknown native container ownership cannot establish all writers stopped');
      assert.equal(report.cleanup?.verified, false, 'retain incomplete attempt for admitted reset');
      const pid = Number(await readFile(childMarker, 'utf8'));
      assert.throws(() => process.kill(pid, 0), { code: 'ESRCH' }, 'retained CLI is reaped');
      const reportBytes = await readFile(join(artifacts, 'holoscan-run.pb'));
      await writeFile(join(output, 'failed-evidence.json'), JSON.stringify({ artifacts, sha256: createHash('sha256').update(reportBytes).digest('hex') }), { mode: 0o600 });
      assert.deepEqual(await sourceState(source), initial);
      console.log(JSON.stringify({ scenario, id, source: initial, reaped: true, cleanupRetained: true, artifacts }));
    } else {
      if (invalidContainer) assert.match(report.executionError, scenario === 'wrong-image' ? /image differs/ : /cgroup parent differs/);
      else assert.equal(report.executionError, '', report.diagnostics.join('; '));
      assert.equal(report.reportError, '');
      assert.equal(report.stopped?.writersStopped, true, report.diagnostics.join('; '));
      assert.equal(report.stopped?.evidenceDurable, true, report.diagnostics.join('; '));
      assert.equal(report.cleanup?.verified, true, report.diagnostics.join('; '));
      assert.equal(report.fileChangesState, doramcap.filesystem.v1.ReportState.REPORT_STATE_COMPLETE);
      if (scenario === 'wrong-image') assert.notEqual(report.imageId, build.record!.imageId);
      else assert.equal(report.imageId, build.record!.imageId);
      if (invalidContainer) await assert.rejects(exec('docker', ['inspect', report.containerId]), /no such/i);
      if (scenario === 'cli-killed') {
        assert.ok(killed);
        assert.equal(Number(report.containerExitCode), 0, 'Engine application completion stays independent of CLI death');
        assert.ok(report.diagnostics.some(note => note.includes('Engine observation continued')));
      }
      assert.equal(report.selection?.application, application);
      if (scenario === 'complete' || scenario === 'keep') {
        assert.equal(report.containerTermination, 'containerExitCode');
        assert.equal(Number(report.containerExitCode), 0);
      }
      assert.ok(report.observedMounts.some(mount => mount.destination === `/workspace/volumes/${data.split('/').at(-1)}`));
      assert.deepEqual(await sourceState(source), initial, 'policy preserves original source facts');
      if (['keep', 'write-discard'].includes(scenario)) {
        assert.ok(wrote, 'case must write through the actual native container');
        const changes = [];
        for await (const line of exportFileChanges(artifacts, report.fileChanges!)) changes.push(JSON.parse(line.toString()));
        const written = changes.filter(row => row.type === 'change' && row.pathDisplay === 'm5-controlled.txt');
        assert.equal(written.length, 2, 'both controlled writes remain in the exported change list');
        assert.ok(written.every(row => row.kind === 'CHANGE_KIND_ADDED' && row.sourceKind === 'SOURCE_KIND_OVERLAY'));
        for (const original of [source, data]) await assert.rejects(access(join(original, 'm5-controlled.txt')), { code: 'ENOENT' });
        const checkpoint = doramcap.filesystem.v1.FileChangeReport.decode(await readFile(join(artifacts, report.fileChanges!.checkpoint!)));
        for (const original of [source, data]) {
          const mapping = checkpoint.overlaySources.find(mapping => Buffer.from(mapping.source ?? []).toString() === original)!;
          assert.ok(mapping, 'capture retains original source identity');
          const kept = join(artifacts, 'kept', String(mapping.index), 'm5-controlled.txt');
          if (scenario === 'keep') assert.match(await readFile(kept, 'utf8'), /m5 controlled write/);
          else await assert.rejects(access(kept), { code: 'ENOENT' });
        }
      }
      const saved = holoscan.HoloscanRunReport.decode(await readFile(join(artifacts, 'holoscan-run.pb')));
      assert.deepEqual(saved.toJSON(), report.toJSON(), 'durable report matches terminal delivery');
      const summary = { scenario, id, source: initial, fileChanges: report.fileChanges, untrackedMounts: report.untrackedMounts, image: report.imageId, exit: report.containerExitCode?.toString(),
        writersStopped: report.stopped?.writersStopped, evidenceDurable: report.stopped?.evidenceDurable,
        cleanupVerified: report.cleanup?.verified, eventStreamHealthy: report.eventStreamHealthy,
        artifacts, diagnostics: report.diagnostics };
      await writeFile(join(output, `${scenario}-summary.json`), JSON.stringify(summary, null, 2), { mode: 0o600 });
      console.log(JSON.stringify(summary));
      if (scenario === 'recover') {
        const previous = JSON.parse(await readFile(join(output, 'failed-evidence.json'), 'utf8'));
        assert.equal(createHash('sha256').update(await readFile(join(previous.artifacts, 'holoscan-run.pb'))).digest('hex'), previous.sha256, 'reset preserves the failed attempt report');
        console.log(JSON.stringify({ recoveredPreviousAttempt: previous.artifacts, newAttempt: id }));
      }
    }
  }
} finally { await owner.close(); }
