import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { mkdir, mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createConsoleServer } from '../src/http.js';
import { availablePort } from '../src/launch.js';
import { runFileFixture } from './run-file-fixture.js';

// Run with the installed Playwright module and Chrome executable as positional arguments.
const [modulePath, executablePath, destination] = process.argv.slice(2);
assert.ok(modulePath && executablePath, 'Pass an installed Playwright module and Chrome executable');
const { chromium } = await import(modulePath);
const output = destination ?? await mkdtemp(join(tmpdir(), 'keep-files-browser-'));
await mkdir(output, { recursive: true });
const run = await runFileFixture();
const port = await availablePort(), origin = `http://127.0.0.1:${port}`;
const server = createConsoleServer({ backend: run.backend, secret: 'secret', origin, assets: resolve('../frontend/dist') });
server.listen(port, '127.0.0.1'); await once(server, 'listening');
const browser = await chromium.launch({ executablePath, headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const errors: string[] = [];
try {
  const context = await browser.newContext({ acceptDownloads: true });
  const session = await context.request.post(`${origin}/api/session`, { headers: { origin, authorization: 'Bearer secret' } });
  assert.equal(session.status(), 204);
  const page = await context.newPage();
  page.setDefaultTimeout(15_000);
  page.on('pageerror', (error: Error) => errors.push(error.message));
  for (const [name, width, height] of [['desktop', 1280, 900], ['narrow', 390, 844]] as const) {
    await page.setViewportSize({ width, height });
    await page.goto(`${origin}/?viewport=${name}#/`);
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page.getByRole('heading', { name: 'Review project', exact: true }).waitFor();
    await page.getByText('Write policy: Keep with the run', { exact: true }).waitFor();
    assert.ok(await page.evaluate('document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1'));
    await page.screenshot({ path: join(output, `${name}-review.png`) });
    await page.goto(`${origin}/#/history/${run.id}`);
    await page.getByText('Write policy: Keep with the run', { exact: true }).waitFor();
    await page.getByText('Kept files destination:', { exact: false }).waitFor();
    await page.getByText(join(run.runRoot, 'kept'), { exact: true }).waitFor();
    const link = page.locator('a[href$="run-file.download?change=0"]');
    await link.waitFor();
    const [download] = await Promise.all([page.waitForEvent('download'), link.click()]);
    const saved = join(output, `${name}-page.html`);
    await download.saveAs(saved);
    assert.deepEqual(await readFile(saved), run.html);
    assert.equal(download.suggestedFilename(), 'page.html');
    assert.equal(await page.evaluate('window.keptHtmlExecuted'), undefined);
    assert.ok(page.url().endsWith(`#/history/${run.id}`));
    assert.ok(await page.evaluate('document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1'));
    await page.getByText('Write policy: Keep with the run', { exact: true }).scrollIntoViewIfNeeded();
    await page.screenshot({ path: join(output, `${name}-result.png`) });
  }
  // A failed copy may never create the folder; its intended destination remains visible.
  await rm(join(run.runRoot, 'kept'), { recursive: true });
  await page.goto(`${origin}/#/history/${run.id}`);
  await page.getByText('Kept files destination:', { exact: false }).waitFor();
  await page.getByText(join(run.runRoot, 'kept'), { exact: true }).waitFor();
  await page.getByText('Kept files destination:', { exact: false }).scrollIntoViewIfNeeded();
  await page.screenshot({ path: join(output, 'missing-folder-destination.png') });
  await page.close();
  assert.deepEqual(errors, []);
  await context.close();
  console.log(JSON.stringify({ browser: 'installed Chromium', views: ['desktop', 'narrow'], attachment: 'HTML downloaded intact without executing', output }));
} finally {
  await browser.close(); server.closeAllConnections(); server.close(); await once(server, 'close'); await run.close();
}
