import { doramcap } from './generated/console';
import type { Snapshot } from './api';
import { isLifecycleStep } from './workflow';
const wire = doramcap.console.v1;
const LIMIT = 4 * 1024 * 1024;
export const decodeStatus = (bytes: Uint8Array) => wire.Status.toObject(wire.Status.decode(bytes), { longs: String, enums: String, defaults: true }) as Snapshot;

/** Four-byte LE framing handles arbitrary HTTP chunk boundaries. */
export class LiveFrames {
  private pending = new Uint8Array(0);
  receive(chunk: Uint8Array, consume: (status: Snapshot) => void) {
    if (chunk.length + this.pending.length > LIMIT * 2 + 8) throw new Error('Live buffer exceeds limit');
    const bytes = new Uint8Array(this.pending.length + chunk.length);
    bytes.set(this.pending); bytes.set(chunk, this.pending.length);
    let offset = 0;
    while (bytes.length - offset >= 4) {
      const length = new DataView(bytes.buffer, bytes.byteOffset + offset, 4).getUint32(0, true);
      if (!length || length > LIMIT) throw new Error('Invalid live frame length');
      if (bytes.length - offset - 4 < length) break;
      consume(decodeStatus(bytes.subarray(offset + 4, offset + 4 + length)));
      offset += 4 + length;
    }
    this.pending = bytes.slice(offset);
  }
}

/** Merge idempotent invocation replacements; keep server-equivalent bounded history. */
export function mergeLive(previous: Snapshot | null, next: Snapshot): Snapshot {
  if (!previous || previous.runId !== next.runId
    || ['IDLE', 'PREPARING', 'REVIEWED'].includes(next.phase)
    || BigInt(next.sequence) < BigInt(previous.sequence)
    || (next.phase === 'STARTING' && previous.phase !== 'STARTING')) return next;
  const steps = next.steps.length ? [...previous.steps] : previous.steps;
  for (const step of next.steps) {
    const index = steps.findIndex(row => row.session === step.session && row.executor === step.executor && row.invocation === step.invocation);
    if (index >= 0) steps[index] = step;
    else {
      if (steps.length === 512) {
        const victim = steps.findIndex(row => row.state !== 'STARTED' && !isLifecycleStep(row));
        if (victim < 0) continue;
        steps.splice(victim, 1);
      }
      steps.push(step);
    }
  }
  let history = previous.metrics;
  if (next.metrics?.length) {
    const metrics = new Map((previous.metrics ?? []).map(metric => [metric.elapsedNs, metric]));
    for (const metric of next.metrics) metrics.set(metric.elapsedNs, metric);
    history = [...metrics.values()].slice(-1200);
  }
  return { ...next, steps, metrics: history, cpuSnapshots: next.cpuSnapshots?.length ? next.cpuSnapshots : previous.cpuSnapshots, cpuGroupNames: next.cpuSnapshots?.length ? next.cpuGroupNames : previous.cpuGroupNames };
}

export async function readLive(consume: (status: Snapshot) => void, signal: AbortSignal) {
  const response = await fetch('/api/live', { credentials: 'same-origin', signal, headers: { accept: 'application/x-protobuf' } });
  if (!response.ok || !response.body || !response.headers.get('content-type')?.startsWith('application/x-protobuf')) throw new Error('Live connection unavailable');
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
