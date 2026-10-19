import { doramcap } from './generated/telemetry.js';
import { KERNEL_SERIES, type Snapshot } from './stream';
export const wire = doramcap.telemetry.v1;
export const WIRE_VERSION = 2;
type Frame = doramcap.telemetry.v1.ISnapshotFrame;
type Entry = doramcap.telemetry.v1.IDictionaryEntry;
function integer(value: number | { toString(): string } | null | undefined): string {
  if (typeof value === 'number' && (!Number.isSafeInteger(value) || value < 0)) throw new Error('Invalid integer');
  const text = (value ?? 0).toString();
  if (!/^\d+$/.test(text) || BigInt(text) > 18446744073709551615n) throw new Error('Invalid uint64');
  return text;
}
function metric(value: number | null | undefined): number | null {
  if (value == null) return null;
  if (!Number.isFinite(value) || value < 0) throw new Error('Invalid metric');
  return value;
}
function entries(values: Entry[] | null | undefined): Snapshot['dictionary'] {
  return (values ?? []).map(d => ({ id: integer(d.id), path: d.path ?? null }));
}
export function decodePacket(bytes: Uint8Array) {
  if (bytes.byteLength > 128 * 1024 * 1024) throw new Error('Frame too large');
  const packet = wire.Packet.decode(bytes);
  if ((!packet.snapshot && !packet.dictionary) || (packet.snapshot && packet.dictionary)) throw new Error('Invalid packet body');
  if ((packet.snapshot ?? packet.dictionary)!.version !== WIRE_VERSION) throw new Error('Unsupported wire version');
  return packet;
}
function snapshot(f: Frame): Snapshot {
  const s = f.snapshot;
  if (!s || s.schemaVersion !== 1) throw new Error('Unsupported snapshot schema');
  const losses = s.taskLosses;
  const result: Snapshot = {
    sequence: integer(s.sequence), capture_unix_ns: integer(s.captureUnixNs), monotonic_end_ns: integer(s.monotonicEndNs),
    dropped_batches: integer(s.droppedBatches), capacity_overflow: integer(s.capacityOverflow), source: f.source ?? '',
    task_collection_enabled: f.taskCollectionEnabled ?? false, task_capacity: integer(f.taskCapacity), session_id: f.sessionId ?? '',
    records: (s.records ?? []).map(r => ({ cgroup_id: ((r.flags ?? 0) & 128) !== 0 ? KERNEL_SERIES : integer(r.cgroupId), cpu: integer(r.cpu), flags: integer(r.flags), runtime_ns: integer(r.runtimeNs) })),
    task_records: (s.taskRecords ?? []).map(r => {
      const i = r.identity;
      if (!i) throw new Error('Missing task identity');
      return { identity: { start_boottime_ns: integer(i.startBoottimeNs), cgroup_id: integer(i.cgroupId), tid: integer(i.tid), tgid: integer(i.tgid), cpu: integer(i.cpu), flags: integer(i.flags) }, charged_runtime_ns: integer(r.chargedRuntimeNs) };
    }),
    task_losses: { dropped_events: integer(losses?.droppedEvents), dropped_runtime_ns: integer(losses?.droppedRuntimeNs), identity_errors: integer(losses?.identityErrors) },
    dictionary: entries(f.dictionary),
  };
  const m = f.viewerIngress;
  if (m) result.viewer_ingress = { total_bytes: integer(m.totalBytes), total_messages: integer(m.totalMessages), bytes_per_second: metric(m.bytesPerSecond), messages_per_second: metric(m.messagesPerSecond), capture_to_receive_ms: metric(m.captureToReceiveMs), mean_capture_to_receive_ms: metric(m.meanCaptureToReceiveMs), invalid_timestamps: integer(m.invalidTimestamps) };
  return result;
}
export function decodeSnapshot(bytes: Uint8Array): Snapshot {
  const f = decodePacket(bytes).snapshot;
  if (!f) throw new Error('Expected snapshot');
  return snapshot(f);
}
// Full checkpoints are bounded and matched by session + dictionary revision.
export class StreamDecoder {
  private dictionary?: { session: string; revision: bigint; entries: Snapshot['dictionary'] };
  dictionaryPackets = 0;
  receive(bytes: Uint8Array): Snapshot | undefined {
    const packet = decodePacket(bytes);
    if (packet.dictionary) {
      const d = packet.dictionary;
      const incoming = { session: d.sessionId ?? '', revision: BigInt(integer(d.revision)), entries: entries(d.entries) };
      if (!this.dictionary || incoming.session !== this.dictionary.session || incoming.revision >= this.dictionary.revision) this.dictionary = incoming;
      this.dictionaryPackets++;
      return;
    }
    const f = packet.snapshot!, s = snapshot(f);
    if (this.dictionary?.session === s.session_id && this.dictionary.revision >= BigInt(integer(f.dictionaryRevision))) s.dictionary = this.dictionary.entries;
    return s;
  }
}
