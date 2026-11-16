import assert from 'node:assert/strict';
import { once } from 'node:events';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
const [modulePath, executablePath, output = '/tmp/pr426-browser'] = process.argv.slice(2);
assert.ok(modulePath && executablePath, 'Pass installed Playwright and Chromium paths');
const { chromium } = await import(modulePath);
import { createConsoleServer } from '../src/http/http.js';
import { availablePort } from '../src/http/launch.js';
import { wire } from '../src/native.js';
import { comparisonFixture, comparisonRuns } from './comparison-fixture.js';
const fixture = await comparisonFixture(true);
const original = fixture.backend.historyAnalysis.bind(fixture.backend);
let mode: 'complete' | 'hold' | 'fail' = 'complete', calls = 0, cancellations = 0;
fixture.backend.historyAnalysis = async (input, signal) => {
  calls++;
  if (mode === 'fail') throw new Error('Internal');
  if (mode === 'hold') await new Promise<void>((resolve, reject) => {
    if (signal.aborted) { cancellations++; reject(signal.reason); return; }
    signal.addEventListener('abort', () => { cancellations++; reject(signal.reason); }, { once: true });
  });
  return original(input, signal);
};
const port = await availablePort(), origin = 'http://127.0.0.1:' + port;
const server = createConsoleServer({ backend: fixture.backend, secret: 'secret', origin,
  assets: resolve('../frontend/dist') });
server.listen(port, '127.0.0.1'); await once(server, 'listening');
const browser = await chromium.launch({ executablePath, headless: true,
  args: ['--no-sandbox', '--disable-dev-shm-usage'] });
await mkdir(output, { recursive: true });
const errors: string[] = [];
try {
  const context = await browser.newContext();
  const response = await context.request.post(origin + '/api/session', { headers: { origin, authorization: 'Bearer secret' } });
  assert.equal(response.status(), 204);
  const page = await context.newPage(); page.setDefaultTimeout(12000);
  await page.addInitScript(`(() => {
    const original = window.fetch;
    window.fetch = async (url, options) => {
      if (!window.delayAnalysisResponse || !String(url).includes('/api/history/analysis')) return original(url, options);
      const response = await original(url, { ...options, signal: undefined });
      window.analysisResponseReady = true;
      await new Promise(resolve => { window.releaseAnalysisResponse = resolve; });
      return response;
    };
  })()`);
  page.on('pageerror', (error: Error) => errors.push(error.message));
  const url = origin + '/#/history/compare/' + comparisonRuns.join('/');
  const panel = page.getByRole('region', { name: 'CPU analysis', exact: true });
  for (const [name, width, height] of [['desktop', 1280, 900], ['narrow', 390, 844]] as const) {
    await page.setViewportSize({ width, height });
    await page.goto(origin + '/?viewport=' + name + '#/history/compare/' + comparisonRuns.join('/'));
    await panel.getByRole('button', { name: 'Run analysis', exact: true }).waitFor();
    assert.equal(calls, name === 'desktop' ? 0 : 1);
    await page.getByText('worker · Exit', { exact: true }).waitFor();
    await page.locator('.comparison-value').filter({ hasText: 'Exit 137; killed' }).first().waitFor();
    await panel.getByRole('button', { name: 'Run analysis', exact: true }).click();
    await panel.getByText('worker · Measured CPU time', { exact: true }).waitFor();
    assert.ok(await panel.locator('.comparison-value').filter({ hasText: '1.000000000 s' }).count() > 0);
    assert.ok(await panel.locator('.comparison-value').filter({ hasText: '2.000000000 s' }).count() > 0);
    assert.ok(await panel.getByText(/Measured: 1.000000000 of 1.000000000 recorded s/).count() > 0);
    assert.ok(await page.evaluate('document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1'));
    await panel.screenshot({ path: output + '/' + name + '-analysis.png' });
  }
  mode = 'hold';
  await panel.getByRole('button', { name: 'Run analysis again', exact: true }).click();
  await panel.getByText('Running…', { exact: true }).waitFor();
  await page.locator('.comparison-value').filter({ hasText: 'Exit 137; killed' }).first().waitFor();
  await panel.getByRole('button', { name: 'Cancel', exact: true }).click();
  await panel.getByRole('button', { name: 'Run analysis', exact: true }).waitFor();
  mode = 'fail';
  await panel.getByRole('button', { name: 'Run analysis', exact: true }).click();
  await panel.getByRole('alert').filter({ hasText: 'This analysis failed.' }).waitFor();
  await page.locator('.comparison-value').filter({ hasText: 'Exit 137; killed' }).first().waitFor();
  mode = 'complete';
  await panel.getByRole('button', { name: 'Run analysis again', exact: true }).click();
  await panel.getByText('worker · Measured CPU time', { exact: true }).waitFor();
  mode = 'hold';
  await panel.getByRole('button', { name: 'Run analysis again', exact: true }).click();
  await panel.getByText('Running…', { exact: true }).waitFor();
  await page.evaluate("location.hash = '#/history/compare/" + [...comparisonRuns].reverse().join('/') + "'");
  await panel.getByRole('button', { name: 'Run analysis', exact: true }).waitFor();
  assert.equal(await panel.getByText('worker · Measured CPU time', { exact: true }).count(), 0);
  mode = 'hold';
  await panel.getByRole('button', { name: 'Run analysis', exact: true }).click();
  await panel.getByText('Running…', { exact: true }).waitFor();
  await page.getByRole('link', { name: 'Clear selection', exact: true }).click();
  await page.getByRole('heading', { name: 'Experiment history', exact: true }).waitFor();
  assert.equal(await panel.count(), 0);
  await page.goto(url);
  await panel.getByRole('button', { name: 'Run analysis', exact: true }).waitFor();
  assert.equal(await panel.getByText('worker · Measured CPU time', { exact: true }).count(), 0);
  assert.ok(cancellations >= 3);
  // A transport that completes after AbortSignal cannot install rows for an older selection.
  mode = 'complete';
  await page.evaluate('window.delayAnalysisResponse = true; window.analysisResponseReady = false');
  await panel.getByRole('button', { name: 'Run analysis', exact: true }).click();
  await page.waitForFunction('window.analysisResponseReady === true');
  await page.evaluate("location.hash = '#/history/compare/" + [...comparisonRuns].reverse().join('/') + "'");
  await panel.getByRole('button', { name: 'Run analysis', exact: true }).waitFor();
  await page.evaluate('window.releaseAnalysisResponse()');
  await page.getByRole('heading', { name: 'Compare runs', exact: true }).waitFor();
  assert.equal(await panel.getByText('worker · Measured CPU time', { exact: true }).count(), 0);
  assert.deepEqual(errors, []);
  console.log(JSON.stringify({ views: ['desktop', '390px'], explicitInvocation: true, realCpuRows: true,
    selectionReplacement: true, staleCompletion: true, cancellation: true, failureRetry: true, navigationAbort: true, returnStartsFresh: true, savedFactsRemainVisible: true,
    calls, cancellations, pageErrors: errors.length, output }));
  await context.close();
} finally {
  await browser.close(); server.closeAllConnections(); server.close(); await once(server, 'close'); await fixture.close();
}
