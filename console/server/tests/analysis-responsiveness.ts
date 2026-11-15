import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { mkdtemp, rm, writeFile, readdir, readlink } from 'node:fs/promises';
import { performance, monitorEventLoopDelay } from 'node:perf_hooks';
import { setTimeout as delay } from 'node:timers/promises';
import { createConsoleServer } from '../src/http/http.js';
import { wire } from '../src/native.js';
import { MEDIA_TYPE, routes } from '../src/generated/transport.js';
import { sessionCookie } from './session.js';
import { comparisonFixture, comparisonCgroup, comparisonSelection } from './comparison-fixture.js';
import { cpuPacket, cpuRecording, dictionary, snapshot } from './cpu-fixture.js';

// Run after npm run build. Exercises the production .js service and worker entry.
const RECORDS_PER_PACKET = 500_000, MATERIALIZED_DEADLINE_MS = 1000, MAIN_LOOP_DEADLINE_MS = 100, CLEANUP_DEADLINE_MS = 5000;
if (process.argv[2] === '--fixture') {
  const records = Array.from({ length: RECORDS_PER_PACKET }, (_, index) => ({
    cgroupId: index ? index + 100 : '9007199254740993', parentId: 1, cpu: 0, flags: 5, runtimeNs: '1000000000',
  }));
  const packet = cpuPacket({ snapshot: { version: 2, sessionId: 'session', dictionaryRevision: 1,
    snapshot: { schemaVersion: 1, sequence: 2, monotonicEndNs: '2000000000', records } } });
  await writeFile(process.argv[3], await cpuRecording([dictionary(comparisonCgroup), snapshot(1, 1_000_000_000n, [0n]), packet, packet, packet, packet]));
  console.log(JSON.stringify({ packetBytes: packet.length }));
  process.exit(0);
}
const directory = await mkdtemp(join(tmpdir(), 'analysis-large-fixture-')), recording = join(directory, 'cpu.mcap');
const generated = await promisify(execFile)(process.execPath, [...process.execArgv, fileURLToPath(import.meta.url), '--fixture', recording]);
const { packetBytes } = JSON.parse(generated.stdout) as { packetBytes: number };
const fixture = await comparisonFixture(true, { recording });
await rm(directory, { recursive: true });
const server = createConsoleServer({ backend: fixture.backend, secret: 'secret', origin: 'http://console.test', assets: '/tmp' });
server.listen(0, '127.0.0.1'); await once(server, 'listening');
const address = server.address(); assert.ok(address && typeof address !== 'string');
const base = 'http://127.0.0.1:' + address.port, cookie = await sessionCookie(base);
const headers = { cookie, origin: 'http://console.test', 'content-type': MEDIA_TYPE };
const comparison = Buffer.from(wire.HistoryComparisonRequest.encode({ sides: comparisonSelection }).finish());
const analysis = Buffer.from(wire.AnalysisRequest.encode({ moduleId: 'cpu', sides: comparisonSelection }).finish());
const opened = async () => {
  for (const fd of await readdir('/proc/self/fd')) if (await readlink('/proc/self/fd/' + fd).catch(() => '') === fixture.firstRecording) return true;
  return false;
};
async function waitForFile(open: boolean) {
  const deadline = performance.now() + CLEANUP_DEADLINE_MS;
  while (await opened() !== open) { assert.ok(performance.now() < deadline, 'recording descriptor reached expected state'); await delay(5); }
}
const loop = monitorEventLoopDelay({ resolution: 5 });
let active = false, settled!: () => void;
const original = fixture.backend.historyAnalysis.bind(fixture.backend);
fixture.backend.historyAnalysis = async (...args) => { active = true; try { return await original(...args); } finally { active = false; settled?.(); } };
try {
  // Actual large-packet decode/reduction, concurrent authenticated materialized HTTP request.
  loop.enable();
  const start = performance.now(), rssBefore = process.memoryUsage().rss;
  const task = fetch(base + routes.historyAnalysis.path, { method: 'POST', headers, body: analysis });
  await waitForFile(true);
  await delay(100); // Allow the worker to enter the large packet, rather than only testing startup.
  const materializedStart = performance.now();
  const materialized = await fetch(base + routes.historyCompare.path, { method: 'POST', headers, body: comparison });
  const saved = wire.HistoryComparison.decode(new Uint8Array(await materialized.arrayBuffer()));
  const materializedMs = performance.now() - materializedStart;
  assert.equal(materialized.status, 200); assert.equal(active, true);
  assert.ok(materializedMs < MATERIALIZED_DEADLINE_MS);
  assert.ok(saved.rows.some(row => row.name === 'service.worker.exit'));
  assert.equal(saved.rows.some(row => row.name?.includes('cpu_')), false);
  const response = await task, result = wire.AnalysisResponse.decode(new Uint8Array(await response.arrayBuffer()));
  assert.equal(response.status, 200);
  assert.ok(result.rows.some(row => row.name === 'service.worker.cpu_seconds' && row.cells?.[0].values?.[0] === '1.000000000'));
  await waitForFile(false);
  const completeMs = performance.now() - start;
  // HTTP disconnect propagates to the production worker; task completion follows fd closure and exit.
  const serverSettled = new Promise<void>(resolve => { settled = resolve; });
  const controller = new AbortController();
  const cancelled = fetch(base + routes.historyAnalysis.path, { method: 'POST', headers, body: analysis, signal: controller.signal });
  const rejection = assert.rejects(cancelled, { name: 'AbortError' });
  await waitForFile(true); await delay(100);
  const cancelStart = performance.now(); controller.abort();
  await rejection;
  await serverSettled;
  const cancelMs = performance.now() - cancelStart;
  assert.ok(cancelMs < CLEANUP_DEADLINE_MS);
  assert.equal(await opened(), false);
  const rssAfter = process.memoryUsage().rss;
  loop.disable();
  assert.ok(loop.max / 1e6 < MAIN_LOOP_DEADLINE_MS, 'main event loop remains responsive during supported large-packet decoding');
  console.log(JSON.stringify({ packetBytes, recordsPerPacket: RECORDS_PER_PACKET, repeatedPackets: 4,
    materializedMs, completeMs, cancelMs, maxMainLoopDelayMs: loop.max / 1e6, rssBefore, rssAfter, maxRssKiB: process.resourceUsage().maxRSS,
    productionWorker: true, descriptorsAfterCancellation: 0 }));
} finally { loop.disable(); server.closeAllConnections(); server.close(); await once(server, 'close'); await fixture.close(); }
