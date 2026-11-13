import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { existsSync, statSync, watch } from 'node:fs';
import { glob, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { homedir, tmpdir } from 'node:os';
import { execFileSync, spawn } from 'node:child_process';
import { once } from 'node:events';
import { createConsoleServer } from '../src/http.js';
import { availablePort, listenConsole } from '../src/launch.js';
import { ConsoleService } from '../src/service.js';
import { wire } from '../src/native.js';
import { filesystem } from '../src/file-changes.js';
import { runFilePath } from '../src/generated/transport.js';
import { sessionCookie } from './session.js';

const DEADLINE_MS = 30000;
const POLL_MS = 25;
const policies = filesystem.WritePolicy;
const records: unknown[][] = [];

async function written(path: string) {
  const watcher = watch(dirname(path));
  try {
    while (!existsSync(path) || statSync(path).size === 0) await once(watcher, 'change', { signal: AbortSignal.timeout(DEADLINE_MS) });
  } finally { watcher.close(); }
}

for (const [form, policy] of [
  ['SYSTEMD', policies.WRITE_POLICY_WRITE_BACK],
  ['COMPOSE', policies.WRITE_POLICY_WRITE_BACK],
  ['SYSTEMD', policies.WRITE_POLICY_KEEP],
  ['SYSTEMD', policies.WRITE_POLICY_DISCARD],
] as const) await test(`real ${form} ${policies[policy]} preserves policy and run-file downloads`, async () => {
  const root = await mkdtemp(join(tmpdir(), 'console-systemd-overlays-'));
  const project = join(root, 'project'), source = join(root, 'source');
  await mkdir(project);
  await mkdir(source);
  await writeFile(join(source, 'existing'), 'before');
  await writeFile(join(source, 'removed'), 'remove');
  const script = `echo modified >${source}/existing; rm ${source}/removed; echo console-change >${source}/result; exec sleep 300`;
  await writeFile(join(project, 'writer.service'), `[Service]\nBindPaths=${source}:${source}\nExecStart=/bin/sh -ec '${script}'\n`);
  if (form === 'COMPOSE') await writeFile(join(project, 'compose.yaml'), `services:\n  writer:\n    image: busybox:latest\n    entrypoint: ["/bin/sh", "-ec"]\n    command: [${JSON.stringify(script)}]\n    volumes: ["${source}:${source}"]\n`);
  const config = join(root, 'console.pbtxt');
  const settings = (policy: number) => `form:${form} ${form === 'COMPOSE' ? 'docker_socket:"/var/run/docker.sock"' : ''} binaries:"/usr/bin" host_shm_access:HOST_SHM_ACCESS_PASSTHROUGH write_policy:${policies[policy]} listen_address:"127.0.0.1:8080" public_origin:"http://127.0.0.1:8080" project_root:"${project}" data_root:"${root}/data" runtime {readiness_seconds:10 stop_seconds:2} ${form === 'SYSTEMD' ? 'apps { id:"output" process {argv:"sh" argv:"-ec" argv:"echo direct-output >/results/output; exec sleep 300"} output_mounts {target:"/results"} }' : ''}`;
  await writeFile(config, settings(policy));
  let { backend } = await ConsoleService.open(config);
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
    const review = wire.Review.decode(await backend.prepare(Buffer.from(wire.PrepareRequest.encode({ profiles: [], systemApps: { ids: form === 'SYSTEMD' ? ['output'] : [] } }).finish())));
    assert.equal(review.writePolicy, policy);
    if (form === 'SYSTEMD') assert.ok(review.warnings.some(note => note.includes('outside declared writable mounts')));
    await backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'overlays' }).finish()));
    await wait(wire.Phase.RUNNING);
    const runRoot = join(root, 'data/runs', review.runId);
    const overlays = JSON.parse(await readFile(join(runRoot, 'overlays/prepared.json'), 'utf8'));
    const overlay = overlays.trees.find((tree: { source: string }) => tree.source === source);
    await written(join(overlay.merged, 'result'));
    await assert.rejects(readFile(join(source, 'result')), { code: 'ENOENT' });
    assert.equal(await readFile(join(source, 'existing'), 'utf8'), 'before');
    assert.equal(await readFile(join(source, 'removed'), 'utf8'), 'remove');
    await backend.stop(Buffer.from(wire.StopRequest.encode({ runId: review.runId }).finish()));
    const finished = await wait(wire.Phase.FINISHED);
    assert.equal(finished.outcome!.boundaryClean, true);
    assert.equal(finished.outcome!.error, '');
    assert.equal(finished.writePolicy, policy);
    assert.equal(finished.fileChanges!.state, filesystem.ReportState.REPORT_STATE_COMPLETE);
    assert.equal(finished.fileChanges!.downloadable, true);
    const saved = filesystem.FileChangeReport.decode(await readFile(join(runRoot, 'filesystem/report.pb')));
    assert.equal(saved.writePolicy, policy);
    assert.ok(saved.overlaySources.length);
    const chunks: Buffer[] = [];
    for await (const chunk of backend.fileChanges(review.runId, new AbortController().signal)) chunks.push(chunk);
    const rows = Buffer.concat(chunks).toString().trim().split('\n').map(line => JSON.parse(line));
    const host = rows.filter(row => row.type === 'change' && row.source === source);
    if (policy === policies.WRITE_POLICY_WRITE_BACK) records.push(host.map(row => [row.pathDisplay, row.kind, row.before?.mode ?? null, row.after?.mode ?? null, ((row.after?.mode ?? 0) & 0o170000) === 0o100000 ? row.after?.size : null]));
    const result = host.find(row => row.pathDisplay === 'result');
    assert.ok(result);
    if (policy === policies.WRITE_POLICY_WRITE_BACK) {
      assert.equal((await readFile(join(source, 'result'), 'utf8')).trim(), 'console-change');
      assert.equal((await readFile(join(source, 'existing'), 'utf8')).trim(), 'modified');
      await assert.rejects(readFile(join(source, 'removed')), { code: 'ENOENT' });
    } else {
      await assert.rejects(readFile(join(source, 'result')), { code: 'ENOENT' });
      assert.equal(await readFile(join(source, 'existing'), 'utf8'), 'before');
      assert.equal(await readFile(join(source, 'removed'), 'utf8'), 'remove');
    }
    if (policy === policies.WRITE_POLICY_KEEP) {
      assert.equal(finished.fileChanges!.keptDirectory, join(runRoot, 'kept'));
      assert.equal((await readFile(join(runRoot, 'kept/0/result'), 'utf8')).trim(), 'console-change');
      await backend.close();
      await writeFile(config, settings(policies.WRITE_POLICY_WRITE_BACK));
      ({ backend } = await ConsoleService.open(config));
      assert.equal((await status()).writePolicy, policy, 'saved policy must survive restart and a config change');
    }
    const { server, origin } = await listenConsole(origin => createConsoleServer({ backend, secret: 'overlay-test', origin, assets: resolve('../frontend/dist') }), await availablePort());
    try {
      const cookie = await sessionCookie(origin, origin, 'overlay-test');
      const download = async (position: string) => fetch(origin + runFilePath(review.runId, wire.RunFileRequest.fromObject({ changePosition: position })), { headers: { cookie } });
      const response = await download(result.position);
      assert.equal(response.status, policy === policies.WRITE_POLICY_KEEP ? 200 : 404);
      if (response.ok) {
        assert.match(response.headers.get('content-disposition')!, /^attachment;/);
        assert.equal((await response.text()).trim(), 'console-change');
      }
      if (form === 'SYSTEMD') {
        const output = rows.find(row => row.type === 'change' && row.source === runRoot && row.pathDisplay === 'output');
        assert.ok(output, 'run output must be reported separately');
        const response = await download(output.position);
        assert.equal(response.status, 200);
        assert.equal((await response.text()).trim(), 'direct-output');
        const ownerHome = process.env.SUDO_USER ? execFileSync('getent', ['passwd', process.env.SUDO_USER], { encoding: 'utf8' }).trim().split(':')[5] : homedir();
        const browsers: string[] = [];
        for await (const browser of glob(join(ownerHome, '.cache/ms-playwright/chromium-*/chrome-linux*/chrome'))) browsers.push(browser);
        assert.ok(browsers.length, 'installed headless Chromium required');
        const browser = spawn(browsers.sort().at(-1)!, ['--headless', '--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage', '--virtual-time-budget=5000', '--dump-dom', `--user-data-dir=${root}/browser`, `${origin}/#access=overlay-test`]);
        let html = '', errors = '';
        browser.stdout.on('data', chunk => { html += chunk; });
        browser.stderr.on('data', chunk => { errors += chunk; });
        assert.equal((await once(browser, 'close'))[0], 0, errors);
        assert.match(html, /File changes/);
        assert.match(html, /Download file changes/);
        assert.match(html, new RegExp(policy === policies.WRITE_POLICY_KEEP ? 'Keep with the run' : policy === policies.WRITE_POLICY_DISCARD ? 'Discard' : 'Write back'));
        if (policy === policies.WRITE_POLICY_KEEP) assert.ok(html.includes(runFilePath(review.runId, { changePosition: result.position })), 'kept file download action must render');
      }
    } finally { server.closeAllConnections(); server.close(); await once(server, 'close'); }
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

await test('equivalent Compose and systemd fixtures produce equivalent host change records', () => {
  assert.equal(records.length, 2);
  assert.deepEqual(records[0], records[1]);
});
