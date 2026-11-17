import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createConsoleServer } from '../src/http/http.js';
import { availablePort } from '../src/http/launch.js';
import { ConsoleService } from '../src/service.js';
import { Store } from '../src/records/store.js';
import { History } from '../src/records/history.js';
import { validateConfig } from '../src/deployment/config.js';
import { filesystem } from '../src/records/file-changes.js';
import { execution, launch, wire, type NativeExecution } from '../src/native.js';

// Run with an installed Playwright module, Chrome executable and optional screenshot directory.
const [modulePath, executablePath, destination] = process.argv.slice(2);
assert.ok(modulePath && executablePath, 'Pass an installed Playwright module and Chrome executable');
const { chromium } = await import(modulePath);
const root = await mkdtemp(join(tmpdir(), 'write-policy-browser-'));
const output = destination ?? join(root, 'screenshots');
await mkdir(output, { recursive: true });
const project = join(root, 'project');
await mkdir(project);
await writeFile(join(project, 'compose.yaml'), 'services: {worker: {image: busybox}}');
for (const profile of ['record', 'live']) await writeFile(join(project, `${profile}.pbtxt`), '');
const { WritePolicy: Policy, SourceKind: Kind } = filesystem;
const config = wire.ConsoleConfig.create({
  projectRoot: project, dataRoot: join(root, 'data'), dockerSocket: '/unused',
  hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH,
  writePolicy: Policy.WRITE_POLICY_KEEP,
  listenAddress: '127.0.0.1:8080', publicOrigin: 'http://console.test',
  apps: [{ id: 'monitor', label: 'Monitor', defaultSelected: true,
    input: { directory: project, files: ['compose.yaml'] },
    configMount: { service: 'monitor', target: '/config/monitor.pbtxt' },
    launchProfiles: ['record', 'live'].map(id => ({ id, label: id === 'record' ? 'Record' : 'Live', config: join(project, `${id}.pbtxt`) })),
  }],
});
validateConfig(config);
const store = await Store.open(config.dataRoot);
const modern = 'a'.repeat(32), legacy = 'b'.repeat(32);
for (const fields of [{ runId: modern, writePolicy: Policy.WRITE_POLICY_DISCARD, systemApps: { ids: ['monitor'], profiles: { monitor: 'record' } } }, { runId: legacy }])
  new History(store).record(wire.Status.create({ phase: wire.Phase.FINISHED, outcome: { execution: 'Completed' }, ...fields }), 1, 2);
let overlays = true;
const engine: NativeExecution = {
  catalog: async () => { throw new Error("Unexpected Holoscan query"); },
    composeProfiles: async () => execution.ComposeProfiles.create(),
  inspect: async () => execution.Inspection.create({
    plan: { services: { worker: { imageId: 'sha256:fixture' } } }, composeSnapshot: { manifestSha256: 'fixture' },
    writableFolders: overlays ? [{ source: join(project, 'bags'), apps: ['worker'], kind: Kind.SOURCE_KIND_OVERLAY }] : [],
  }),
  start: async () => {}, stop: async () => {}, poll: () => execution.ExecutionUpdate.create(), close: async () => {},
};
const backend = new ConsoleService(config, engine, store);
const requests: InstanceType<typeof wire.PrepareRequest>[] = [];
const prepare = backend.prepare.bind(backend);
backend.prepare = input => { requests.push(wire.PrepareRequest.decode(input)); return prepare(input); };
const port = await availablePort(), origin = `http://127.0.0.1:${port}`;
const server = createConsoleServer({ backend, secret: 'secret', origin, assets: resolve('../frontend/dist') });
server.listen(port, '127.0.0.1'); await once(server, 'listening');
const browser = await chromium.launch({ executablePath, headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const errors: string[] = [];
try {
  const context = await browser.newContext();
  assert.equal((await context.request.post(`${origin}/api/session`, { headers: { origin, authorization: 'Bearer secret' } })).status(), 204);
  const page = await context.newPage();
  page.setDefaultTimeout(15_000);
  page.on('pageerror', (error: Error) => errors.push(error.message));
  const ready = async () => { await page.getByRole('heading', { name: 'Review project', exact: true }).waitFor(); await page.locator('button:has-text("Start run"):enabled').waitFor(); };
  const radio = (name: string) => page.getByRole('radio', { name: new RegExp(`^${name}`) });
  for (const [name, width, height] of [['desktop', 1280, 900], ['narrow', 390, 844]] as const) {
    await page.setViewportSize({ width, height });
    await page.goto(`${origin}/?viewport=${name}#/`);
    await page.getByRole('button', { name: 'Continue', exact: true }).click(); await ready();
    assert.equal(await radio('Keep with the run').isChecked(), true);
    // A policy change keeps the app/profile choices accepted by PR392's Review flow.
    const profile = page.getByRole('combobox', { name: 'Monitor profile' });
    await profile.scrollIntoViewIfNeeded(); await profile.focus();
    await profile.press('ArrowDown'); await profile.press('End'); await profile.press('Enter'); await ready();
    assert.equal(requests.at(-1)!.systemApps!.profiles!.monitor, 'live');
    await radio('Write back').check(); await ready();
    assert.equal(requests.at(-1)!.writePolicy, Policy.WRITE_POLICY_WRITE_BACK);
    assert.deepEqual(requests.at(-1)!.systemApps!.profiles, { monitor: 'live' });
    assert.match(await page.getByRole('combobox', { name: 'Monitor profile' }).innerText(), /Live/);
    assert.ok(await page.evaluate('document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1'));
    await page.screenshot({ path: join(output, `${name}-review.png`) });
    // App preferences survive reload; each New run starts with the project's policy.
    await page.reload(); await page.getByRole('button', { name: 'Continue', exact: true }).click(); await ready();
    assert.equal(await radio('Keep with the run').isChecked(), true);
    assert.match(await page.getByRole('combobox', { name: 'Monitor profile' }).innerText(), /Live/);
    await page.getByRole('checkbox', { name: 'Monitor', exact: true }).uncheck(); await ready();
    await page.goto(`${origin}/#/history/${modern}`);
    await page.getByRole('button', { name: 'Run again', exact: true }).click(); await ready();
    assert.equal(await radio('Discard').isChecked(), true);
    assert.equal(await page.getByRole('checkbox', { name: 'Monitor', exact: true }).isChecked(), true);
    assert.deepEqual(requests.at(-1)!.systemApps!.profiles, { monitor: 'record' });
    assert.equal(requests.at(-1)!.baselineRunId, modern);
    // Legacy entries use configured app defaults and historical Write back, never tab/cache choices.
    await page.getByRole('checkbox', { name: 'Monitor', exact: true }).uncheck(); await ready();
    await page.goto(`${origin}/#/history/${legacy}`);
    await page.getByRole('button', { name: 'Run again', exact: true }).click(); await ready();
    assert.equal(await radio('Write back').isChecked(), true);
    assert.equal(await page.getByRole('checkbox', { name: 'Monitor', exact: true }).isChecked(), true);
    assert.equal(requests.at(-1)!.systemApps, null);
    assert.equal(requests.at(-1)!.writePolicy, Policy.WRITE_POLICY_WRITE_BACK);
    assert.equal(requests.at(-1)!.baselineRunId, legacy);
    await page.screenshot({ path: join(output, `${name}-legacy-rerun.png`) });
  }
  overlays = false;
  await page.reload(); await page.getByRole('button', { name: 'Continue', exact: true }).click(); await ready();
  assert.equal(await page.getByRole('radio').count(), 0);
  assert.deepEqual(errors, []);
  await context.close();
  console.log(JSON.stringify({ browser: 'installed Chromium', views: ['desktop', 'narrow'], behavior: 'policy changes, app preferences, modern/legacy Run again and no-overlay visibility', output }));
} finally {
  await browser.close(); server.closeAllConnections(); server.close(); await once(server, 'close'); await backend.close(); await rm(root, { recursive: true, force: true });
}
