import { wire, Phase, routes, type Snapshot } from './protocol';
import { evictionIndex } from './workflow';
import { MEDIA_TYPE, FramedMessages } from './generated/transport';

export const decodeStatus = (bytes: Uint8Array) => wire.Status.decode(bytes);
export class LiveFrames extends FramedMessages<Snapshot> {
  constructor() { super(decodeStatus); }
}

/** Merge idempotent invocation replacements; keep server-equivalent bounded history. */
export function mergeLive(previous: Snapshot | null, next: Snapshot): Snapshot {
  if (!previous || previous.runId !== next.runId
    || [Phase.IDLE, Phase.PREPARING, Phase.REVIEWED].includes(next.phase)
    || BigInt(String(next.sequence)) < BigInt(String(previous.sequence))
    || (next.phase === Phase.STARTING && previous.phase !== Phase.STARTING)) return next;
  const steps = next.steps.length ? [...previous.steps] : previous.steps;
  for (const step of next.steps) {
    const index = steps.findIndex(row => row.session === step.session && row.executor === step.executor && String(row.invocation) === String(step.invocation));
    if (index >= 0) steps[index] = step;
    else {
      if (steps.length === 512) {
        const victim = evictionIndex(steps, step);
        if (victim < 0) continue;
        steps.splice(victim, 1);
      }
      steps.push(step);
    }
  }
  let history = previous.metrics;
  if (next.metrics?.length) {
    const metrics = new Map((previous.metrics ?? []).map(metric => [String(metric.elapsedNs), metric]));
    for (const metric of next.metrics) metrics.set(String(metric.elapsedNs), metric);
    history = [...metrics.values()].slice(-1200);
  }
  return wire.Status.create({ ...next, steps, metrics: history, cpuSnapshots: next.cpuSnapshots?.length ? next.cpuSnapshots : previous.cpuSnapshots, cpuGroupNames: next.cpuSnapshots?.length ? next.cpuGroupNames : previous.cpuGroupNames });
}

export async function readLive(consume: (status: Snapshot) => void, signal: AbortSignal, fetcher: typeof fetch = fetch) {
  const response = await fetcher(routes.live.path, { credentials: 'same-origin', signal, headers: { accept: MEDIA_TYPE } });
  if (!response.ok || !response.body || !response.headers.get('content-type')?.startsWith(MEDIA_TYPE)) throw new Error('Live connection unavailable');
  const reader = response.body.getReader();
  const frames = new LiveFrames();
  let snapshot: Snapshot | null = null;
  try {
    while (!signal.aborted) {
      const { done, value } = await reader.read();
      if (done) throw new Error('Live connection ended');
      frames.receive(value, next => { snapshot = mergeLive(snapshot, next); consume(snapshot); });
    }
  } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
}
