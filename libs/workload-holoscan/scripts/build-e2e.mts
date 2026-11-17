import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { basename, isAbsolute, join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { capabilities, execution, holoscan, launch } from '../../../console/server/src/native.js';
import { sourceState } from '../../../console/server/src/records/source.js';

const [cli, source, socket, baseImage, output] = process.argv.slice(2);
assert.ok([cli, source, socket, output].every(value => value && isAbsolute(value)) && baseImage,
  'usage: tsx build-e2e.mts /absolute/holoscan /absolute/first-pipeline-fixture /absolute/docker.sock native-base-image /absolute/new-output');
await mkdir(output, { mode: 0o700 });
const project = join(output, 'source');
await cp(source, project, { recursive: true, filter: path => !['build', '.git'].includes(basename(path)) });
await writeFile(join(project, 'metadata.json'), JSON.stringify({ $schema: 'urn:holoscan:module:v2', module: {
  name: 'holoscan-build-probe', version: '1.0.0', support_level: 'community', description: 'Build lifecycle fixture',
  authors: [{ name: 'Fixture', affiliation: 'Fixture' }], license: 'Apache-2.0', language: ['Python'], namespace: { python: 'holoscan_build_probe' },
  holoscan_sdk: { minimum_required_version: '5.0.0', tested_versions: ['5.0.0'] }, platforms: ['x86_64'], tags: ['fixture'],
  operator_names: ['Sample'], dependencies: [], dockerfile: 'Dockerfile',
} }));
await writeFile(join(project, 'pyproject.toml'), `[tool.holoscan]\ndocker-run-args=["--init"]\n[tool.holoscan.base-images]\nx86_64=${JSON.stringify(baseImage)}\n`);
const cmake = join(project, 'CMakeLists.txt');
await writeFile(cmake, `if(EXISTS "\${CMAKE_SOURCE_DIR}/.fail-build")\n  message(FATAL_ERROR "DORAMCAP_EXPECTED_BUILD_FAILURE")\nendif()\nif(EXISTS "\${CMAKE_SOURCE_DIR}/.cancel-build")\n  message(STATUS "DORAMCAP_CANCEL_READY")\n  execute_process(COMMAND "\${CMAKE_COMMAND}" -E sleep 300)\nendif()\n${await readFile(cmake, 'utf8')}`);
const git = async (...args: string[]) => promisify(execFile)('git', ['-C', project, ...args]);
await writeFile(join(project, '.gitignore'), '/build/\n/data/\n/.cache/\n/.cupy/\n/.fail-build\n/.cancel-build\n');
await git('init', '-q', '-b', 'main');
await git('add', '.');
await git('-c', 'user.name=Build Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'Initial fixture');
const data = join(output, "data 'quoted'\nline");
await mkdir(data, { mode: 0o700 });
const selection = holoscan.Selection.create({ application: 'first_pipeline', language: 'python', mode: 'finite' });
const store = holoscan.BuildStoreRequest.create({ root: join(data, 'builds'), key: { repositoryRoot: project, selection } });
const request = async () => holoscan.BuildRequest.create({ attempt: { id: randomUUID(), store, source: await sourceState(project) }, cli,
  readinessSeconds: 30, stopSeconds: 15, hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH });
let owner = await capabilities.open(socket, 1);
const results: Record<string, unknown> = {};
const OBSERVATION_INTERVAL_MS = 20;
const BUILD_OBSERVATION_DEADLINE_MS = 120_000;
async function waitForMarker(marker: string, attemptId: string) {
  const deadline = Date.now() + BUILD_OBSERVATION_DEADLINE_MS;
  while (Date.now() < deadline) {
    const update = owner.poll();
    const progress = update.runId === attemptId ? update.buildProgress : null;
    if (progress?.attemptId === attemptId && progress.logs?.stdout && (await readFile(progress.logs.stdout, 'utf8').catch(() => '')).includes(marker)) return progress;
    if (update.runId === attemptId && update.buildOutcome) throw new Error(`Build ended before ${marker}`);
    await delay(OBSERVATION_INTERVAL_MS);
  }
  throw new Error(`Build did not reach ${marker}`);
}
try {
  // The unchanged SDK application is built by the native CLI using the fixture's native base-image default.
  const successful = await owner.build(await request()); results.success = successful.toJSON();
  assert.equal(successful.publicationError, '');
  assert.equal(successful.attempt!.result!.exitCode, 0, JSON.stringify(successful.toJSON()));
  assert.equal(successful.attempt!.result!.cleanupVerified, true, JSON.stringify(successful.toJSON()));
  assert.equal(successful.attempt!.result!.imageError, '', JSON.stringify(successful.toJSON()));
  assert.ok(successful.attempt!.result!.record!.imageId);
  const disk = await readFile(successful.logs!.stdout!);
  assert.ok(disk.length);
  assert.equal(String(owner.poll().buildProgress!.stdoutBytes), String(disk.length));
  assert.equal(owner.poll().buildOutcome!.attempt!.id, successful.attempt!.id);
  const lookup = await owner.buildLookup(store);
  assert.deepEqual(lookup.record, successful.attempt!.result!.record);
  assert.deepEqual(lookup.logs, successful.logs);
  const reviewed = (await owner.inspect(execution.RunRequest.create({
    run: { id: 'review', repositoryRoot: project, artifacts: join(output, 'review'), readinessSeconds: 30, stopSeconds: 15, hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH },
    holoscan: { buildStoreRoot: store.root, plan: { cli, selection }, currentSource: await sourceState(project), buildRecord: { selection, imageId: 'sha256:forged' } },
  }))).holoscan!;
  assert.deepEqual(reviewed.rejections, []);
  assert.equal(reviewed.build!.state, holoscan.BuildState.BUILD_STATE_FRESH);
  assert.equal(reviewed.snapshot!.imageId, lookup.record!.imageId);
  results.review = holoscan.Preparation.fromObject(reviewed).toJSON();
  await owner.close(); owner = await capabilities.open(socket, 1);
  assert.deepEqual((await owner.buildLookup(store)).record, lookup.record);
  await writeFile(join(project, 'source-revision.txt'), 'committed edit after Build\n');
  await git('add', 'source-revision.txt');
  await git('-c', 'user.name=Build Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'Change source after Build');
  const changed = (await owner.inspect(execution.RunRequest.create({
    run: { id: 'review-changed', repositoryRoot: project, artifacts: join(output, 'review-changed'), readinessSeconds: 30, stopSeconds: 15, hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH },
    holoscan: { buildStoreRoot: store.root, plan: { cli, selection }, currentSource: await sourceState(project) },
  }))).holoscan!;
  assert.equal(changed.build!.state, holoscan.BuildState.BUILD_STATE_REBUILD_NEEDED);
  results.changed = holoscan.Preparation.fromObject(changed).toJSON();

  // A native CMake failure replaces success and retains complete CLI logs.
  await writeFile(join(project, '.fail-build'), '');
  const failed = await owner.build(await request()); results.failure = failed.toJSON();
  assert.notEqual(failed.attempt!.result!.exitCode, 0);
  assert.equal(failed.attempt!.result!.cleanupVerified, true, JSON.stringify(failed.toJSON()));
  assert.equal(failed.attempt!.result!.record, null);
  assert.equal((await owner.buildLookup(store)).record, null);
  assert.match(await readFile(failed.logs!.stderr!, 'utf8'), /DORAMCAP_EXPECTED_BUILD_FAILURE/);
  await rm(join(project, '.fail-build'));

  // Stop and close occur after the real container reports its waiting point.
  await writeFile(join(project, '.cancel-build'), '');
  const stoppedRequest = await request(), pending = owner.build(stoppedRequest);
  const progress = await waitForMarker('DORAMCAP_CANCEL_READY', stoppedRequest.attempt!.id!);
  assert.ok(Number(progress.stdoutBytes) > 0);
  const other = await capabilities.open(socket, 1);
  try { await assert.rejects(other.buildLookup(store), /another run owns this repository/); } finally { await other.close(); }
  await owner.stop(stoppedRequest.attempt!.id!);
  const stopped = await pending; results.stopped = stopped.toJSON();
  assert.equal(stopped.attempt!.result!.cancelled, true);
  assert.equal(stopped.attempt!.result!.cleanupVerified, true, JSON.stringify(stopped.toJSON()));
  assert.equal((await owner.buildLookup(store)).record, null);

  const closingRequest = await request(), closing = owner.build(closingRequest);
  await waitForMarker('DORAMCAP_CANCEL_READY', closingRequest.attempt!.id!);
  await owner.close();
  const closed = await closing; results.closed = closed.toJSON();
  assert.equal(closed.attempt!.result!.cancelled, true);
  assert.equal(closed.attempt!.result!.cleanupVerified, true, JSON.stringify(closed.toJSON()));
  owner = await capabilities.open(socket, 1);
  assert.equal((await owner.buildLookup(store)).latest!.id, closed.attempt!.id);
  assert.equal((await owner.buildLookup(store)).record, null);
  await rm(join(project, '.cancel-build'));
  const recovered = await owner.build(await request()); results.recovered = recovered.toJSON();
  assert.equal(recovered.attempt!.result!.cleanupVerified, true);
  assert.ok((await owner.buildLookup(store)).record);
  console.log(`Native Build, failure, Stop, close, reopen and latest-record evidence passed: ${output}`);
} finally {
  await owner.close();
  await writeFile(join(output, 'report.json'), JSON.stringify(results, null, 2));
}
