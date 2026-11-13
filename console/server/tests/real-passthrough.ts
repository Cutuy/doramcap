/** Build unprivileged; elevate only this test for Docker, systemd and OverlayFS. */
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { createServer } from 'node:net';
import { once } from 'node:events';
import { glob, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { existsSync, statSync, watch } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { homedir, tmpdir } from 'node:os';
import { execFileSync, spawn } from 'node:child_process';
import { ConsoleService } from '../src/service.js';
import { wire } from '../src/native.js';
import { filesystem } from '../src/file-changes.js';
import { createConsoleServer } from '../src/http.js';
import { availablePort, listenConsole } from '../src/launch.js';

const DEADLINE = 30000;
async function written(path: string) {
  const watcher = watch(dirname(path));
  try { while (!existsSync(path) || !statSync(path).size) await once(watcher, 'change', { signal: AbortSignal.timeout(DEADLINE) }); }
  finally { watcher.close(); }
}

for (const form of ['COMPOSE', 'SYSTEMD']) await test(`${form}: live sockets pass through beside a tracked overlay`, { timeout: 90000 }, async () => {
  // Arrange: a socket created outside the run, plus an unrelated writable data folder.
  const root = await mkdtemp(join(tmpdir(), 'console-passthrough-'));
  const project = join(root, 'project'), live = join(root, 'live'), tracked = join(root, 'tracked');
  for (const folder of [project, live, tracked]) await mkdir(folder);
  const socket = createServer(peer => peer.once('data', () => peer.end('host-live')));
  socket.listen(join(live, 'socket')); await once(socket, 'listening');
  const script = `printf ping | /usr/bin/socat - UNIX-CONNECT:${live}/socket >${live}/direct; echo tracked >${tracked}/result; exec sleep 300`;
  if (form === 'SYSTEMD') await writeFile(join(project, 'writer.service'), `[Service]\nBindPaths=${live}:${live}:norbind\nBindPaths=${tracked}:${tracked}\nExecStart=/bin/sh -ec '${script}'\n`);
  else await writeFile(join(project, 'compose.yaml'), `services:\n  writer:\n    image: doramcap/mcap-recorder:local\n    entrypoint: ["/bin/sh", "-ec"]\n    command: [${JSON.stringify(script)}]\n    volumes: ["/usr:/usr:ro", "${live}:${live}", "${tracked}:${tracked}"]\n`);
  const config = join(root, 'console.pbtxt');
  await writeFile(config, `form:${form} ${form === 'COMPOSE' ? 'docker_socket:"/var/run/docker.sock"' : ''} binaries:"/usr/bin" host_shm_access:HOST_SHM_ACCESS_PASSTHROUGH write_policy:WRITE_POLICY_DISCARD passthrough_folders:"${live}" listen_address:"127.0.0.1:8080" public_origin:"http://127.0.0.1:8080" project_root:"${project}" data_root:"${root}/data" runtime {readiness_seconds:10 stop_seconds:2}`);
  const { backend } = await ConsoleService.open(config);
  let reviewed: InstanceType<typeof wire.Review> | undefined;
  const prepare = backend.prepare.bind(backend);
  backend.prepare = async input => { const bytes = await prepare(input); reviewed = wire.Review.decode(bytes); return bytes; };
  const { server, origin } = await listenConsole(origin => createConsoleServer({ backend, secret: 'passthrough-test', origin, assets: resolve('../frontend/dist') }), await availablePort());
  let browser: ReturnType<typeof spawn> | undefined, browserClosed: Promise<unknown> | undefined;
  let cdp: WebSocket | undefined;
  let send: (method: string, params?: object) => Promise<any>;
  let pageLoaded: () => Promise<unknown>;
  let evaluate: (expression: string) => Promise<any>;
  const waitText = (text: string) => `new Promise((resolve, reject) => {
    const ready = () => { if (document.body?.innerText.includes(${JSON.stringify(text)})) { observer.disconnect(); clearTimeout(timer); resolve(document.body.innerText); } };
    const observer = new MutationObserver(ready); observer.observe(document, {childList:true, subtree:true, characterData:true});
    const timer = setTimeout(() => { observer.disconnect(); reject(new Error('Missing: ' + ${JSON.stringify(text)})); }, ${DEADLINE}); ready();
  })`;
  const waitPhase = async (phase: number) => {
    const deadline = Date.now() + DEADLINE;
    let status = wire.Status.decode(await backend.status());
    while (status.phase !== phase && Date.now() < deadline) { await new Promise(done => setTimeout(done, 25)); status = wire.Status.decode(await backend.status()); }
    assert.equal(status.phase, phase, JSON.stringify(status)); return status;
  };
  try {
    if (form === 'SYSTEMD') {
      // Use installed Chromium and its DevTools protocol; no browser package is added.
      const home = process.env.SUDO_USER ? execFileSync('getent', ['passwd', process.env.SUDO_USER], { encoding: 'utf8' }).trim().split(':')[5] : homedir();
      const browsers: string[] = [];
      for await (const path of glob(join(home, '.cache/ms-playwright/chromium-*/chrome-linux*/chrome'))) browsers.push(path);
      assert.ok(browsers.length, 'installed Chromium required');
      browser = spawn(browsers.sort().at(-1)!, ['--headless', '--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage', '--remote-debugging-port=0', `--user-data-dir=${root}/browser`, 'about:blank']);
      browserClosed = once(browser, 'close');
      const endpoint = await new Promise<string>(done => { let errors = ''; browser!.stderr!.on('data', chunk => { errors += chunk; const match = errors.match(/DevTools listening on (ws:\/\/[^\s]+)/); if (match) done(match[1]); }); });
      const target = await (await fetch(`http://127.0.0.1:${new URL(endpoint).port}/json/new?about:blank`, { method: 'PUT' })).json() as { webSocketDebuggerUrl: string };
      cdp = new WebSocket(target.webSocketDebuggerUrl); await once(cdp, 'open');
      let sequence = 0;
      const pending = new Map<number, { resolve(value: any): void; reject(reason: unknown): void }>();
      let loaded: (() => void) | undefined;
      cdp.addEventListener('message', event => {
        const message = JSON.parse(String(event.data));
        if (message.method === 'Page.loadEventFired') loaded?.();
        if (message.id) { const request = pending.get(message.id); pending.delete(message.id); if (message.error) request?.reject(message.error); else request?.resolve(message.result); }
      });
      send = (method, params = {}) => new Promise((resolve, reject) => { const id = ++sequence; pending.set(id, { resolve, reject }); cdp!.send(JSON.stringify({ id, method, params })); });
      pageLoaded = () => new Promise<void>(done => { loaded = done; });
      evaluate = async expression => { const value = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true, userGesture: true }); assert.equal(value.exceptionDetails, undefined, JSON.stringify(value)); return value.result.value; };
      await send('Page.enable');
      await send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 900, deviceScaleFactor: 1, mobile: false });
      const loadedPage = pageLoaded(); await send('Page.navigate', { url: `${origin}/#access=passthrough-test` }); await loadedPage;
      await evaluate(waitText('Continue'));
      await evaluate(`Array.from(document.querySelectorAll('button')).find(button => button.textContent === 'Continue').click()`);
      const reviewText = await evaluate(waitText('not tracked: passed through by passthrough_folders'));
      assert.match(reviewText, /Review project/); assert.ok(reviewText.includes(live)); assert.match(reviewText, /Discard/);
      assert.match(reviewText, /Writable folders/);
      assert.match(reviewText, /During the run/); assert.match(reviewText, /After the run/);
      assert.match(reviewText, /Passed through/); assert.match(reviewText, /Already on the host; no change list/);
      await send('Page.bringToFront');
      await evaluate(`document.querySelector('.folder-setting button').click()`);
      await evaluate(waitText('Copied'));
      assert.equal(await evaluate(`document.querySelector('.folder-setting code').textContent`), `passthrough_folders: ${JSON.stringify(live)}`);
      for (const width of [1280, 390]) {
        await send('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: false });
        await evaluate('new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)))');
        assert.equal(await evaluate('document.documentElement.scrollWidth <= innerWidth'), true, `Review overflows at ${width}`);
        const { cssContentSize } = await send('Page.getLayoutMetrics');
        const screenshot = await send('Page.captureScreenshot', { captureBeyondViewport: true, clip: { x: 0, y: 0, width, height: Math.min(cssContentSize.height, await evaluate('Math.ceil(document.querySelector(".writable-folders").getBoundingClientRect().bottom + scrollY + 20)')), scale: 1 } });
        await writeFile(resolve(`../../target/writable-folders-review-${width}.png`), Buffer.from(screenshot.data, 'base64'));
      }
    } else await backend.prepare(Buffer.from(wire.PrepareRequest.encode({ systemApps: { ids: [] } }).finish()));
    assert.ok(reviewed);
    assert.ok(reviewed.warnings.some(note => note.includes(live) && note.includes('not tracked: passed through by passthrough_folders')));
    assert.equal(reviewed.writableFolders.find(folder => folder.source === live)?.kind, filesystem.SourceKind.SOURCE_KIND_PASSTHROUGH);
    assert.equal(reviewed.writableFolders.find(folder => folder.source === tracked)?.kind, filesystem.SourceKind.SOURCE_KIND_OVERLAY);
    assert.ok(reviewed.writableFolders.every(folder => folder.apps?.length));
    // Act: both apps reach the host socket and keep their ordinary data isolated.
    await backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: reviewed.preparedId, requestKey: 'passthrough' }).finish()));
    await waitPhase(wire.Phase.RUNNING);
    await written(join(live, 'direct')); assert.equal(await readFile(join(live, 'direct'), 'utf8'), 'host-live');
    const runRoot = join(root, 'data/runs', reviewed.runId);
    const views = JSON.parse(await readFile(join(runRoot, 'overlays/prepared.json'), 'utf8'));
    assert.deepEqual(views.trees.map((tree: { source: string }) => tree.source), [tracked]);
    await written(join(views.trees[0].merged, 'result'));
    await assert.rejects(readFile(join(tracked, 'result')), { code: 'ENOENT' });
    await backend.stop(Buffer.from(wire.StopRequest.encode({ runId: reviewed.runId }).finish()));
    const finished = await waitPhase(wire.Phase.FINISHED);
    // Assert: only the overlay enters the change list; discard leaves live writes alone.
    assert.equal(finished.outcome!.error, ''); assert.equal(finished.outcome!.boundaryClean, true);
    const chunks: Buffer[] = []; for await (const chunk of backend.fileChanges(reviewed.runId, new AbortController().signal)) chunks.push(chunk);
    const rows = Buffer.concat(chunks).toString().trim().split('\n').map(line => JSON.parse(line));
    assert.ok(rows.some(row => row.type === 'change' && row.source === tracked && row.pathDisplay === 'result'));
    assert.ok(!rows.some(row => row.type === 'change' && row.source === live));
    assert.match(JSON.stringify(rows), /not tracked: passed through by passthrough_folders/);
    assert.equal(await readFile(join(live, 'direct'), 'utf8'), 'host-live');
    await assert.rejects(readFile(join(tracked, 'result')), { code: 'ENOENT' });
    if (form === 'SYSTEMD') {
      const loadedPage = pageLoaded!(); await send!('Page.reload'); await loadedPage;
      const resultText = await evaluate!(waitText('not tracked: passed through by passthrough_folders'));
      assert.match(resultText, /File changes/); assert.ok(resultText.includes(live)); assert.match(resultText, /Discard/);
      assert.match(resultText, /Writable folders/);
      for (const width of [1280, 390]) {
        await send!('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: false });
        await evaluate!('new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)))');
        assert.equal(await evaluate!('document.documentElement.scrollWidth <= innerWidth'), true, `Result overflows at ${width}`);
        const { cssContentSize } = await send!('Page.getLayoutMetrics');
        const screenshot = await send!('Page.captureScreenshot', { captureBeyondViewport: true, clip: { x: 0, y: 0, width, height: cssContentSize.height, scale: 1 } });
        await writeFile(resolve(`../../target/writable-folders-result-${width}.png`), Buffer.from(screenshot.data, 'base64'));
      }
    }
  } finally {
    if (cdp) { cdp.send(JSON.stringify({ id: 999999, method: 'Browser.close' })); await browserClosed; cdp.close(); }
    server.closeAllConnections(); server.close(); await once(server, 'close');
    await backend.close(); await new Promise<void>(done => socket.close(() => done()));
    await rm(root, { recursive: true, force: true });
  }
});
