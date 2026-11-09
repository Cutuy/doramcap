import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { doramcap } from '../src/generated/console.js';
import { artifactPath, FramedMessages, MEDIA_TYPE, routes } from '../src/generated/transport.js';

const wire = doramcap.console.v1;
const workflow = doramcap.workflow.v1;
const RUN_DEADLINE_MS = 100_000;
const POLL_MS = 250;
const LIVE_DEADLINE_MS = 15_000;
const REQUIRED_LIVE_FRAMES = 4;
const OBSERVATION_MS = 3000;
const MCAP_MAGIC = Buffer.from('894d434150300d0a', 'hex');
const CPU_APP = 'cpu-monitor';
const LIVE_APP = 'live-bridge';

const summary = (status: doramcap.console.v1.Status) => ({
  phase: wire.Phase[status.phase], runId: status.runId,
  steps: status.steps.map(event => ({ step: event.step, state: workflow.Event.State[event.state ?? 0], error: event.error })),
  outcome: status.outcome && wire.Outcome.toObject(wire.Outcome.create(status.outcome), { enums: String, longs: String }),
  cpuAvailable: status.cpuAvailable, metricsIncomplete: status.metricsIncomplete,
  liveStreams: status.liveStreams.map(stream => stream.id),
});

/** Check an existing Console's CPU playground through its normal authenticated HTTP API.
 * Starts a run and requests verified Stop even if Start's response or live checks fail.
 * The launch file supplies credentials; returned reports contain no access token or cookie. */
export async function checkPlayground(options: { launchFile: string; expectedSteps?: string[] }) {
  const text = await readFile(options.launchFile, 'utf8');
  const link = text.match(/^Open: (https?:\S+)$/m)?.[1];
  assert.ok(link, 'Launch file must contain an Open: access link');
  const access = new URL(link);
  const token = new URLSearchParams(access.hash.slice(1)).get('access');
  assert.ok(token && !access.username && !access.password, 'Invalid Console access link');
  const session = await fetch(`${access.origin}/api/session`, {
    method: 'POST', signal: AbortSignal.timeout(RUN_DEADLINE_MS),
    headers: { Origin: access.origin, Authorization: `Bearer ${token}` },
  });
  assert.equal(session.status, 204, 'Console authentication failed');
  const cookie = session.headers.get('set-cookie')?.split(';')[0];
  assert.ok(cookie, 'Console did not return a session cookie');
  const headers = { Origin: access.origin, Cookie: cookie, 'Content-Type': MEDIA_TYPE };
  async function request<T>(path: string, decode: (bytes: Uint8Array) => T, body?: Uint8Array): Promise<T> {
    const response = await fetch(access.origin + path, {
      method: body ? 'POST' : 'GET', headers, signal: AbortSignal.timeout(RUN_DEADLINE_MS),
      body: body ? new Blob([new Uint8Array(body)]) : undefined,
    });
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (!response.ok) throw new Error(`${path}: HTTP ${response.status}: ${wire.ConsoleError.decode(bytes).message}`);
    return decode(bytes);
  }
  async function waitFor(phases: number[]) {
    const deadline = Date.now() + RUN_DEADLINE_MS;
    while (Date.now() < deadline) {
      const status = await request(routes.current.path, wire.Status.decode);
      if (phases.includes(status.phase)) return status;
      if ([wire.Phase.BLOCKED, wire.Phase.FINISHED].includes(status.phase)) throw new Error(JSON.stringify(summary(status)));
      await delay(POLL_MS);
    }
    throw new Error('Timed out waiting for Console run state');
  }
  const review = await request(routes.prepare.path, wire.Review.decode, wire.PrepareRequest.encode({
    profiles: [], systemApps: { ids: [CPU_APP, LIVE_APP], profiles: { [CPU_APP]: 'record' } },
  }).finish());
  let failure: unknown;
  let frameCount = 0;
  try {
    await request(routes.start.path, wire.StartResponse.decode, wire.StartRequest.encode({
      preparedId: review.preparedId, requestKey: randomUUID(),
    }).finish());
    await waitFor([wire.Phase.RUNNING]);
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), LIVE_DEADLINE_MS);
    try {
      const response = await fetch(access.origin + routes.live.path, { headers, signal: controller.signal });
      assert.equal(response.status, 200, 'Live stream failed');
      const reader = response.body!.getReader();
      const decoder = new FramedMessages(wire.LiveMessage.decode);
      try {
        while (frameCount < REQUIRED_LIVE_FRAMES) {
          const { value, done } = await reader.read();
          if (done) break;
          decoder.receive(value, message => { if (message.frame?.length) frameCount++; });
        }
      } finally { await reader.cancel(); }
    } finally { clearTimeout(timeout); controller.abort(); }
    assert.ok(frameCount >= REQUIRED_LIVE_FRAMES, `Only ${frameCount} live frames received`);
    await delay(OBSERVATION_MS);
  } catch (error) { failure = error; }
  let finished: doramcap.console.v1.Status;
  try {
    await request(routes.stop.path, wire.StopResponse.decode, wire.StopRequest.encode({ runId: review.runId }).finish());
    finished = await waitFor([wire.Phase.FINISHED, wire.Phase.BLOCKED]);
  } catch (error) {
    if (failure) throw new AggregateError([failure, error], 'Playground check and cleanup both failed');
    throw error;
  }
  if (failure) throw failure;
  assert.equal(finished.phase, wire.Phase.FINISHED, JSON.stringify(summary(finished)));
  assert.equal(finished.outcome?.writersStopped, true);
  assert.equal(finished.outcome?.recordingFinalized, true);
  assert.equal(finished.outcome?.boundaryClean, true);
  for (const step of options.expectedSteps ?? [])
    assert.ok(finished.steps.some(event => event.step === step && event.state === workflow.Event.State.SUCCEEDED), step);
  const artifact = await fetch(access.origin + artifactPath(review.runId, 'cpu'), {
    headers, signal: AbortSignal.timeout(RUN_DEADLINE_MS),
  });
  assert.equal(artifact.status, 200, 'CPU recording download failed');
  let recordingBytes = 0, head = Buffer.alloc(0), tail = Buffer.alloc(0);
  for await (const chunk of artifact.body!) {
    recordingBytes += chunk.length;
    if (head.length < MCAP_MAGIC.length) head = Buffer.concat([head, chunk]).subarray(0, MCAP_MAGIC.length);
    tail = Buffer.concat([tail, chunk]).subarray(-MCAP_MAGIC.length);
  }
  assert.ok(recordingBytes >= MCAP_MAGIC.length * 2, 'CPU recording is empty');
  assert.deepEqual(head, MCAP_MAGIC, 'Invalid MCAP header');
  assert.deepEqual(tail, MCAP_MAGIC, 'MCAP recording is not sealed');
  return { ...summary(finished), frameCount, recordingBytes };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { values } = parseArgs({ options: {
    'launch-file': { type: 'string' }, output: { type: 'string' },
    'expect-step': { type: 'string', multiple: true },
  } });
  assert.ok(values['launch-file'], 'Usage: check:playground -- --launch-file FILE [--output FILE] [--expect-step ID]');
  const report = JSON.stringify(await checkPlayground({
    launchFile: values['launch-file'], expectedSteps: values['expect-step'],
  }), null, 2) + '\n';
  if (values.output) await writeFile(values.output, report, { mode: 0o600 });
  console.log(report.trimEnd());
}
