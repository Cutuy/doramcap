import { decode } from '@msgpack/msgpack';
import { KERNEL_SERIES, type Snapshot } from './stream';
export const WIRE_VERSION = 1;
function tuple(value: unknown, length: number): unknown[] {
  if (!Array.isArray(value) || value.length !== length) throw new Error('Invalid wire tuple');
  return value;
}
function list(value: unknown): unknown[] {
  if (!Array.isArray(value)) throw new Error('Invalid wire list');
  return value;
}
function integer(value: unknown): string {
  if (typeof value === 'bigint' && value >= 0n) return value.toString();
  if (typeof value === 'number' && Number.isSafeInteger(value) && value >= 0) return String(value);
  throw new Error('Invalid wire integer');
}
function text(value: unknown): string {
  if (typeof value !== 'string') throw new Error('Invalid wire string');
  return value;
}
function flag(value: unknown): boolean {
  if (typeof value !== 'boolean') throw new Error('Invalid wire boolean');
  return value;
}
function metric(value: unknown): number | null {
  if (value === null) return null;
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0) throw new Error('Invalid metric');
  return value;
}
// Strings below are an in-memory adapter for existing BigInt delta calculations.
// Wire integers are compact binary integers, never decimal strings.
function unpack(bytes: Uint8Array): unknown {
  if (bytes.byteLength > 128 * 1024 * 1024) throw new Error('Frame too large');
  return decode(bytes, { useBigInt64: true, maxArrayLength: 1_048_576, maxMapLength: 0, maxStrLength: 1_048_576 });
}
export function decodeSnapshot(bytes: Uint8Array, decoded?: unknown): Snapshot {
  if (bytes.byteLength > 128 * 1024 * 1024) throw new Error('Frame too large');
  const f = tuple(decoded ?? unpack(bytes), 11);
  if (f[0] !== WIRE_VERSION) throw new Error('Unsupported wire version');
  const s = tuple(f[1], 10);
  if (s[0] !== 1) throw new Error('Unsupported snapshot schema');
  const losses = tuple(s[9], 3);
  const result: Snapshot = {
    sequence: integer(s[1]), capture_unix_ns: integer(s[2]), monotonic_end_ns: integer(s[4]),
    dropped_batches: integer(s[5]), capacity_overflow: integer(s[6]), source: text(f[2]),
    task_collection_enabled: flag(f[3]), task_capacity: integer(f[4]), session_id: text(f[5]),
    records: list(s[7]).map(row => {
      const r = tuple(row, 7);
      return { cgroup_id: (Number(integer(r[3])) & 128) !== 0 ? KERNEL_SERIES : integer(r[0]), cpu: integer(r[2]), flags: integer(r[3]), runtime_ns: integer(r[4]) };
    }),
    task_records: list(s[8]).map(row => {
      const r = tuple(row, 2), i = tuple(r[0], 6);
      return { identity: { start_boottime_ns: integer(i[0]), cgroup_id: integer(i[1]), tid: integer(i[2]), tgid: integer(i[3]), cpu: integer(i[4]), flags: integer(i[5]) }, charged_runtime_ns: integer(r[1]) };
    }),
    task_losses: { dropped_events: integer(losses[0]), dropped_runtime_ns: integer(losses[1]), identity_errors: integer(losses[2]) },
    dictionary: list(f[9]).map(row => { const d = tuple(row, 3); return { id: integer(d[0]), path: d[2] === null ? null : text(d[2]) }; }),
  };
  if (f[10] !== null) {
    const m = tuple(f[10], 7);
    result.viewer_ingress = { total_bytes: integer(m[0]), total_messages: integer(m[1]), bytes_per_second: metric(m[2]), messages_per_second: metric(m[3]), capture_to_receive_ms: metric(m[4]), mean_capture_to_receive_ms: metric(m[5]), invalid_timestamps: integer(m[6]) };
  }
  return result;
}

// One full checkpoint retained; session + revision prevent stale name reuse.
export class StreamDecoder {
  private dictionary?: { session: string; revision: bigint; entries: Snapshot['dictionary'] };
  dictionaryPackets = 0;
  receive(bytes: Uint8Array): Snapshot | undefined {
    const raw = unpack(bytes);
    if (Array.isArray(raw) && raw.length === 6) {
      const d = tuple(raw, 6);
      if (d[0] !== WIRE_VERSION) throw new Error('Unsupported dictionary version');
      integer(d[3]); integer(d[5]);
      const incoming = { session: text(d[1]), revision: BigInt(integer(d[2])), entries: list(d[4]).map(row => {
        const entry = tuple(row, 3); integer(entry[1]);
        return { id: integer(entry[0]), path: entry[2] === null ? null : text(entry[2]) };
      }) };
      if (!this.dictionary || incoming.session !== this.dictionary.session || incoming.revision >= this.dictionary.revision) this.dictionary = incoming;
      this.dictionaryPackets++;
      return;
    }
    const f = tuple(raw, 11), snapshot = decodeSnapshot(bytes, f);
    if (this.dictionary?.session === snapshot.session_id && this.dictionary.revision >= BigInt(integer(f[6]))) {
      snapshot.dictionary = this.dictionary.entries;
    }
    return snapshot;
  }
}
