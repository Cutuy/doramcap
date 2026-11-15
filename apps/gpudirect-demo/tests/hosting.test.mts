import assert from 'node:assert/strict';
import { mkdtemp, rename, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { start } from '../deploy/serve.mts';

type App = Awaited<ReturnType<typeof start>>;
const TEST_TIMEOUT_MS = 10_000;

async function withApp(check: (app: App, base: string, release: (outcome?: string) => Promise<void>) => Promise<void>) {
  const directory = await mkdtemp(join(tmpdir(), 'gpu-page-'));
  const binary = join(directory, 'fixture.cjs');
  const gate = join(directory, 'release');
  const release = async (outcome = 'success') => {
    await writeFile(`${gate}.pending`, outcome);
    await rename(`${gate}.pending`, gate);
  };
  let app: App | undefined;
  try {
    // A file notification releases the simulated GPU job after the page is observed.
    await writeFile(binary, `#!${process.execPath}
const fs = require('node:fs'), path = require('node:path');
const [output, mode] = process.argv.slice(2);
const gate = path.join(path.dirname(output), 'release');
let released = false;
const watcher = fs.watch(path.dirname(output), finish);
function finish() {
  if (released || !fs.existsSync(gate)) return;
  released = true;
  watcher.close();
  if (fs.readFileSync(gate, 'utf8') === 'failure') process.exit(1);
  fs.writeFileSync(output, mode);
  console.log('payload_mib_s=1 cpu_pct_one_core=1 wall_s=1 verified=true');
}
finish();
`, { mode: 0o700 });
    app = await start(binary, directory, 0);
    const address = app.server.address();
    assert.ok(address && typeof address !== 'string');
    await check(app, `http://127.0.0.1:${address.port}`, release);
  } finally {
    await release();
    await app?.stop().catch(() => {});
    await rm(directory, { recursive: true, force: true });
  }
}

async function readPage(reader: ReadableStreamDefaultReader<Uint8Array>, until?: string) {
  let text = '';
  while (true) {
    const { done, value } = await reader.read();
    if (done) { assert.ok(!until || text.includes(until), text); return text; }
    text += Buffer.from(value).toString('utf8');
    if (until && text.includes(until)) return text;
  }
}

test('the first page shows preparation and receives verified results without a reload', { timeout: TEST_TIMEOUT_MS }, async () => {
  await withApp(async (app, base, release) => {
    // Arrange: the GPU job cannot finish until the test releases it.
    assert.equal((await fetch(`${base}/health`)).status, 503);
    // Act
    const response = await fetch(base);
    assert.equal(response.status, 200);
    const reader = response.body!.getReader();
    const preparing = await readPage(reader, 'Preparing GPU recordings');
    // Assert: the user sees progress before any results can exist.
    assert.ok(!preparing.includes('<table>'));
    assert.equal((await fetch(`${base}/mapped.mcap`)).status, 503);
    // Act: complete all three recordings while retaining the first response.
    await release();
    const results = await readPage(reader);
    await app.completion;
    // Assert: the same response contains working downloads and verified results.
    assert.match(results, /Every payload byte verified/);
    for (const mode of ['mapped', 'staged', 'io-only']) {
      assert.ok(results.includes(`./${mode}.mcap`));
      assert.equal(await (await fetch(`${base}/${mode}.mcap`)).text(), mode);
    }
    assert.equal((await fetch(`${base}/health`)).status, 200);
    await app.stop();
  });
});

test('a recording failure ends the preparing page without advertising results', { timeout: TEST_TIMEOUT_MS }, async () => {
  await withApp(async (app, base, release) => {
    // Arrange
    const response = await fetch(base);
    const reader = response.body!.getReader();
    await readPage(reader, 'Preparing GPU recordings');
    const failed = assert.rejects(app.completion, /mapped:/);
    // Act
    await release('failure');
    const result = await readPage(reader);
    await failed;
    // Assert
    assert.match(result, /GPU recording failed/);
    assert.doesNotMatch(result, /<table>|Full measurements|Every payload byte verified/);
    assert.equal((await fetch(`${base}/health`)).status, 503);
    await assert.rejects(app.stop(), /mapped:/);
  });
});

test('leaving the preparing page does not interrupt recording or prevent shutdown', { timeout: TEST_TIMEOUT_MS }, async () => {
  await withApp(async (app, base, release) => {
    // Arrange
    const response = await fetch(base);
    const reader = response.body!.getReader();
    await readPage(reader, 'Preparing GPU recordings');
    // Act
    await reader.cancel();
    await release();
    await app.completion;
    // Assert: every mode completed despite the browser leaving during initialization.
    for (const mode of ['mapped', 'staged', 'io-only']) {
      assert.equal(await (await fetch(`${base}/${mode}.mcap`)).text(), mode);
    }
    await app.stop();
    assert.equal(app.server.listening, false);
  });
});

test('stopping during preparation closes the page without publishing partial results', { timeout: TEST_TIMEOUT_MS }, async () => {
  await withApp(async (app, base, release) => {
    // Arrange
    const response = await fetch(base);
    const reader = response.body!.getReader();
    await readPage(reader, 'Preparing GPU recordings');
    // Act: shutdown still drains the active job, then closes its HTTP connections.
    const page = readPage(reader).catch(() => 'connection closed');
    const stopped = app.stop();
    await release();
    await stopped;
    // Assert: a stopped workload never advertises the complete result set.
    assert.doesNotMatch(await page, /<table>|Full measurements|Every payload byte verified/);
    assert.equal(app.server.listening, false);
  });
});
