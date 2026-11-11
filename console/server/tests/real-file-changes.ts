import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { ConsoleService } from '../src/service.js';
import { wire } from '../src/native.js';

/** Real Docker/OverlayFS qualification. Build unprivileged, elevate this test only. */
test('real Compose writes retain the same file report and complete export after Console restart', { timeout: 90000 }, async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-real-file-changes-'));
  const project = join(root, 'project'), source = join(root, 'source');
  await mkdir(project); await mkdir(source);
  await writeFile(join(source, 'modified'), 'before');
  await writeFile(join(source, 'deleted'), 'remove me');
  await writeFile(join(project, 'compose.yaml'), `services:
  worker:
    image: doramcap/mcap-recorder:local
    entrypoint: [/bin/sh, -c]
    command: ["printf added > /tracked/added; printf after > /tracked/modified; rm /tracked/deleted; touch /tracked/ready; trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done"]
    volumes: ["${source}:/tracked"]
    healthcheck:
      test: [CMD-SHELL, "test -f /tracked/ready"]
      interval: 1s
      timeout: 1s
      retries: 20
`);
  await writeFile(join(root, 'secret'), 'qualification-only', { mode: 0o600 });
  const config = join(root, 'console.pbtxt');
  await writeFile(config, `host_shm_access: HOST_SHM_ACCESS_BLOCKED
    listen_address: "127.0.0.1:8080" public_origin: "http://127.0.0.1:8080"
    project_root: "${project}" data_root: "${root}/data"
    docker_socket: "/var/run/docker.sock" access_secret_file: "${root}/secret"
    runtime { readiness_seconds: 30 stop_seconds: 3 }`);
  let backend: ConsoleService | undefined;
  let clean = true;
  try {
    ({ backend } = await ConsoleService.open(config));
    const review = wire.Review.decode(await backend.prepare(Buffer.from(wire.PrepareRequest.encode({ systemApps: { ids: [] } }).finish())));
    // Act
    clean = false;
    await backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'file-changes' }).finish()));
    let status = wire.Status.decode(await backend.status());
    const deadline = Date.now() + 60000;
    while (status.phase === wire.Phase.STARTING && Date.now() < deadline) { await delay(100); status = wire.Status.decode(await backend.status()); }
    assert.equal(status.phase, wire.Phase.RUNNING, JSON.stringify(status));
    // Assert: source writes remain isolated until stop/finalization.
    assert.equal(await readFile(join(source, 'modified'), 'utf8'), 'before');
    await assert.rejects(readFile(join(source, 'added')));
    // Act
    await backend.stop(Buffer.from(wire.StopRequest.encode({ runId: review.runId }).finish()));
    while ([wire.Phase.STARTING, wire.Phase.RUNNING, wire.Phase.STOPPING].includes(status.phase) && Date.now() < deadline) { await delay(100); status = wire.Status.decode(await backend.status()); }
    clean = status.outcome?.boundaryClean === true;
    // Assert
    assert.ok(clean, JSON.stringify(status));
    assert.ok(status.fileChanges?.downloadable, JSON.stringify(status.fileChanges));
    const lines = [];
    for await (const bytes of backend.fileChanges(review.runId, new AbortController().signal)) lines.push(JSON.parse(bytes.toString()));
    const changes = lines.filter(row => row.type === 'change' && row.category === 'HOST_BIND');
    for (const [path, kind] of [['added', 'ADDED'], ['modified', 'MODIFIED'], ['deleted', 'DELETED']])
      assert.ok(changes.some(row => row.pathDisplay === path && row.kind === kind), `${path}: ${JSON.stringify(changes)}`);
    assert.equal(lines.at(-1).type, 'complete');
    assert.equal(await readFile(join(source, 'modified'), 'utf8'), 'after');
    await assert.rejects(readFile(join(source, 'deleted')));
    // Act: restart and read the persisted current result and History detail.
    await backend.close();
    ({ backend } = await ConsoleService.open(config));
    const current = wire.Status.decode(await backend.status());
    const past = wire.HistoryDetail.decode(await backend.historyRun(review.runId));
    // Assert
    assert.deepEqual(current.fileChanges, status.fileChanges);
    assert.deepEqual(past.status?.fileChanges, status.fileChanges);
    const restarted = [];
    for await (const bytes of backend.fileChanges(review.runId, new AbortController().signal)) restarted.push(JSON.parse(bytes.toString()));
    assert.deepEqual(restarted, lines);
    console.log(`Verified ${changes.length} host changes, report persistence and complete restart export.`);
  } finally {
    await backend?.close();
    if (clean) await rm(root, { recursive: true, force: true });
    else console.error(`Qualification resources retained for recovery: ${root}`);
  }
});
