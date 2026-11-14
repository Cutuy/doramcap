import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { glob, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { homedir, tmpdir } from 'node:os';
import { execFileSync, spawn } from 'node:child_process';
import { once } from 'node:events';
import { resolve } from 'node:path';
import { createConsoleServer } from '../src/http.js';
import { availablePort, listenConsole } from '../src/launch.js';
import { ConsoleService } from '../src/service.js';
import { wire } from '../src/native.js';
import { filesystem } from '../src/file-changes.js';

const DEADLINE_MS = 30000;
const POLL_MS = 25;

const records: unknown[][] = [];
for (const form of ['SYSTEMD', 'COMPOSE']) await test(`a real ${form} run publishes File changes through Console and its download`, async () => {
  const root = await mkdtemp(join(tmpdir(), 'console-systemd-overlays-'));
  const project = join(root, 'project'), source = join(root, 'source');
  await mkdir(project);
  await mkdir(source);
  await writeFile(join(project, 'writer.service'), `[Service]\nBindPaths=${source}:${source}\nExecStart=/bin/sh -ec 'echo console-change >${source}/result; exec sleep 300'\n`);
  if (form === 'COMPOSE') await writeFile(join(project, 'compose.yaml'), `services:\n  writer:\n    image: busybox:latest\n    entrypoint: [\"/bin/sh\", \"-ec\"]\n    command: [\"echo console-change >${source}/result; exec sleep 300\"]\n    volumes: [\"${source}:${source}\"]\n`);
  const config = join(root, 'console.pbtxt');
  await writeFile(config, `form:${form} ${form === 'COMPOSE' ? 'docker_socket:"/var/run/docker.sock"' : ''} binaries:"/usr/bin" host_shm_access:HOST_SHM_ACCESS_PASSTHROUGH listen_address:"127.0.0.1:8080" public_origin:"http://127.0.0.1:8080" project_root:"${project}" data_root:"${root}/data" runtime {readiness_seconds:10 stop_seconds:2}`);
  const { backend } = await ConsoleService.open(config);
  const status = async () => wire.Status.decode(await backend.status());
  const wait = async (phase: number) => {
    const deadline = Date.now() + DEADLINE_MS;
    let current = await status();
    while (current.phase !== phase && Date.now() < deadline) {
      await new Promise(done => setTimeout(done, POLL_MS));
      current = await status();
    }
    assert.equal(current.phase, phase, JSON.stringify(wire.Status.toObject(current, { enums: String })));
    return current;
  };
  try {
    const review = wire.Review.decode(await backend.prepare(Buffer.from(wire.PrepareRequest.encode({ profiles: [], systemApps: { ids: [] } }).finish())));
    if (form === 'SYSTEMD') assert.ok(review.warnings.some(note => note.includes('outside declared writable mounts')));
    await backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'overlays' }).finish()));
    await wait(wire.Phase.RUNNING);
    await assert.rejects(readFile(join(source, 'result')), { code: 'ENOENT' });
    await backend.stop(Buffer.from(wire.StopRequest.encode({ runId: review.runId }).finish()));
    const finished = await wait(wire.Phase.FINISHED);
    assert.equal(finished.outcome!.boundaryClean, true);
    assert.equal(finished.outcome!.error, '');
    assert.equal(finished.fileChanges!.state, filesystem.ReportState.REPORT_STATE_COMPLETE);
    assert.equal(finished.fileChanges!.downloadable, true);
    assert.ok(Number(finished.fileChanges!.report!.categories!.find(category => category.category === filesystem.StorageCategory.STORAGE_CATEGORY_HOST_BIND)!.counts!.added) > 0);
    const chunks: Buffer[] = [];
    for await (const chunk of backend.fileChanges(review.runId, new AbortController().signal)) chunks.push(chunk);
    const exported = Buffer.concat(chunks).toString();
    assert.match(exported, /result/);
    records.push(exported.trim().split('\n').map(line => JSON.parse(line)).filter(row => row.type === 'change' && row.category === 'STORAGE_CATEGORY_HOST_BIND').map(row => [row.pathDisplay, row.kind, row.before?.mode ?? null, row.after?.mode ?? null, ((row.after?.mode ?? 0) & 0o170000) === 0o100000 ? row.after?.size : null]));
    if (form === 'SYSTEMD') {
      const ownerHome = process.env.SUDO_USER ? execFileSync('getent', ['passwd', process.env.SUDO_USER], { encoding: 'utf8' }).trim().split(':')[5] : homedir();
      const browsers: string[] = [];
      for await (const browser of glob(join(ownerHome, '.cache/ms-playwright/chromium-*/chrome-linux*/chrome'))) browsers.push(browser);
      assert.ok(browsers.length, 'installed headless Chromium required');
      const { server, origin } = await listenConsole(origin => createConsoleServer({ backend, secret: 'overlay-test', origin, assets: resolve('../frontend/dist') }), await availablePort());
      try {
        const browser = spawn(browsers.sort().at(-1)!, ['--headless', '--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage', '--virtual-time-budget=5000', '--dump-dom', `--user-data-dir=${root}/browser`, `${origin}/#access=overlay-test`]);
        let html = '', errors = '';
        browser.stdout.on('data', chunk => { html += chunk; });
        browser.stderr.on('data', chunk => { errors += chunk; });
        assert.equal((await once(browser, 'close'))[0], 0, errors);
        assert.match(html, /File changes/);
        assert.match(html, /Download file changes/);
        assert.match(html, /result/);
      } finally { server.closeAllConnections(); server.close(); await once(server, 'close'); }
    }
    assert.equal((await readFile(join(source, 'result'), 'utf8')).trim(), 'console-change');
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

await test('equivalent Compose and systemd fixtures produce equivalent host change records', () => {
  assert.equal(records.length, 2);
  assert.deepEqual(records[0], records[1]);
});
