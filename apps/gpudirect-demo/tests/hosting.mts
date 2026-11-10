import assert from 'node:assert/strict';
import { access, mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { start } from '../deploy/serve.mts';

if (process.argv.length !== 3) throw new Error('usage: node tests/hosting.mts /absolute/path/gpudirect-demo');
const binary = resolve(process.argv[2]);
const directory = await mkdtemp(join(tmpdir(), 'gpudirect-hosting-'));
try {
  // Arrange / Act: real CUDA recordings and an ephemeral HTTP endpoint.
  const results = join(directory, 'success');
  const app = await start(binary, results, 0);
  const address = app.server.address();
  assert(address && typeof address !== 'string');
  const base = `http://127.0.0.1:${address.port}`;
  try {
    await app.completion;
    // Assert: readiness exposes verified files and measurements for every mode.
    assert.equal((await fetch(`${base}/health`)).status, 200);
    const page = await (await fetch(base)).text();
    for (const mode of ['mapped', 'staged', 'io-only']) {
      assert(page.includes(`./${mode}.mcap`));
      const download = await fetch(`${base}/${mode}.mcap`);
      assert.equal(download.status, 200);
      assert.deepEqual(Buffer.from(await download.arrayBuffer()), await readFile(join(results, `${mode}.mcap`)));
    }
    assert.equal((await fetch(`${base}/unknown`)).status, 404);
    assert.match(await readFile(join(results, 'stats.txt'), 'utf8'), /verified=true/);
    assert.equal(await (await fetch(`${base}/architecture.svg`)).text(), await readFile(new URL('../architecture.svg', import.meta.url), 'utf8'));
  } finally { await app.stop(); }
  assert.equal(await readFile(join(results, 'shutdown.complete'), 'utf8'), 'complete\n');
  console.log('Verified GPU outputs, HTTP downloads and graceful shutdown: pass');

  // Act / Assert: a failed recording cannot advertise readiness or completion.
  const failedResults = join(directory, 'failure');
  const failed = await start('/bin/false', failedResults, 0);
  await assert.rejects(failed.completion, /mapped:/);
  await assert.rejects(failed.stop(), /mapped:/);
  await assert.rejects(access(join(failedResults, 'shutdown.complete')), { code: 'ENOENT' });
  console.log('Recording failure leaves no completion marker: pass');

  // Act / Assert: cancellation drains the active CUDA job without publishing a partial run.
  const cancelledResults = join(directory, 'cancelled');
  const cancelled = await start(binary, cancelledResults, 0);
  await cancelled.stop();
  await assert.rejects(access(join(cancelledResults, 'shutdown.complete')), { code: 'ENOENT' });
  await assert.rejects(access(join(cancelledResults, 'stats.txt')), { code: 'ENOENT' });
  console.log('Startup cancellation drains CUDA and leaves no completion marker: pass');
} finally { await rm(directory, { recursive: true, force: true }); }
