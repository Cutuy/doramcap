import { doramcap } from './generated/console';
import { topCpuGroups } from './cpu';

const live = doramcap.console.live.v1;
const telemetry = doramcap.telemetry.v1;
const fault = doramcap.fault.v1;
export const Kind = live.Kind;
export const Health = fault.Health;
export type Frame = doramcap.console.live.v1.Frame;
type Stream = doramcap.console.v1.ILiveStream;
type SnapshotFrame = doramcap.telemetry.v1.ISnapshotFrame;
type FaultState = doramcap.fault.v1.IState;
const integer = (value: unknown) => BigInt(String(value ?? 0));
const hex = (bytes?: Uint8Array | null) => Array.from(bytes ?? [], (byte) => byte.toString(16).padStart(2, '0')).join('');
const MAX_HISTORY = 1200;
const MAX_NAME_CHARS = 128;

export type CpuSample = { seconds: number; percent: number | null };
export type CpuView = {
  history: CpuSample[];
  cores: number;
  groups: { id: string; name: string; percent: number }[];
  sequence?: string;
  droppedBatches?: string;
  capacityOverflow?: string;
};
export type FaultView = {
  sources: number;
  connected: number;
  conditions: { key: string; source: string; component: string; resource: string; code: string; severity: number; health: number }[];
  stale: boolean;
};
export type LiveView = { cpu?: CpuView; faults?: FaultView; gaps: number };

/** Total core percent from BPF root rows (flag 16) of two snapshots of one session. */
export function totalCpu(before: SnapshotFrame, after: SnapshotFrame): { percent: number | null; cores: number } {
  const roots = (frame: SnapshotFrame) => {
    const snapshot = frame.snapshot;
    if (!snapshot || integer(snapshot.capacityOverflow) > 0n) return undefined;
    const rows = (snapshot.records ?? []).filter((row) => (Number(row.flags) & 16) !== 0);
    if (!rows.length || rows.some((row) => (Number(row.flags) & 1) === 0 || (Number(row.flags) & (64 | 128)) !== 0 || row.cpu === 0xffffffff)) return undefined;
    const counters = new Map(rows.map((row) => [Number(row.cpu), integer(row.runtimeNs)]));
    return counters.size === rows.length ? counters : undefined;
  };
  const old = roots(before), next = roots(after);
  const unavailable = { percent: null, cores: next?.size ?? 0 };
  const elapsed = integer(after.snapshot?.monotonicEndNs) - integer(before.snapshot?.monotonicEndNs);
  if (!old || !next || old.size !== next.size || elapsed <= 0n || before.sessionId !== after.sessionId) return unavailable;
  let runtime = 0n;
  for (const [cpu, value] of next) {
    const previous = old.get(cpu);
    if (previous == null || value < previous) return unavailable;
    runtime += value - previous;
  }
  return { percent: (Number(runtime) / Number(elapsed)) * 100, cores: next.size };
}

/** CPU monitor packets: snapshots and dictionaries share one codec. */
class CpuStreams {
  private frames: SnapshotFrame[] = [];
  private names = new Map<string, string>();
  private history: CpuSample[] = [];
  private cores = 0;
  receive(payload: Uint8Array) {
    const packet = telemetry.Packet.decode(payload);
    const entries = packet.dictionary?.entries ?? packet.snapshot?.dictionary ?? [];
    for (const entry of entries) {
      const name = String(entry.path ?? '').split('/').filter(Boolean).at(-1)?.slice(0, MAX_NAME_CHARS);
      if (name && name !== '.' && name !== '..') this.names.set(String(entry.id), name);
    }
    const frame = packet.snapshot;
    if (!frame?.snapshot) return;
    const previous = this.frames.at(-1);
    if (previous && integer(frame.snapshot.sequence) <= integer(previous.snapshot?.sequence)) return;
    this.frames = [...this.frames, frame].slice(-2);
    if (!previous) return;
    const total = totalCpu(previous, frame);
    this.cores = total.cores;
    this.history = [...this.history, { seconds: Number(integer(frame.snapshot.captureUnixNs) / 1_000_000n) / 1000, percent: total.percent }].slice(-MAX_HISTORY);
  }
  view(): CpuView {
    const last = this.frames.at(-1)?.snapshot;
    return {
      history: this.history,
      cores: this.cores,
      groups: topCpuGroups(this.frames.map((frame) => frame.snapshot!)).map((group) => ({ ...group, name: this.names.get(group.id) ?? '' })),
      sequence: last?.sequence?.toString(),
      droppedBatches: last?.droppedBatches?.toString(),
      capacityOverflow: last?.capacityOverflow?.toString(),
    };
  }
}

/** Fault Watchdog publications: snapshots replace state; contiguous updates amend it. */
class FaultStreams {
  private epoch = '';
  private revision = 0n;
  private stale = true;
  private sources = new Map<string, { connected: boolean; conditions: Map<number, FaultState> }>();
  receive(payload: Uint8Array) {
    const publication = fault.AggregationPublication.decode(payload);
    if (publication.snapshot) {
      const snapshot = publication.snapshot;
      this.epoch = hex(snapshot.epoch);
      this.revision = integer(snapshot.revision);
      this.stale = false;
      this.sources = new Map(
        (snapshot.sources ?? []).map((source) => [
          hex(source.source),
          { connected: !!source.connected, conditions: new Map((source.conditions ?? []).map((state) => [Number(state.key?.condition ?? 0), state])) },
        ]),
      );
      return;
    }
    const batch = publication.updates;
    if (!batch || hex(batch.epoch) !== this.epoch) {
      this.stale = true;
      return;
    }
    for (const update of batch.updates ?? []) {
      const revision = integer(update.revision);
      if (revision <= this.revision) continue;
      // A revision gap marks the view stale until the next snapshot.
      if (revision !== this.revision + 1n) this.stale = true;
      this.revision = revision;
      const id = hex(update.key?.source ?? update.source);
      const source = this.sources.get(id) ?? { connected: true, conditions: new Map() };
      this.sources.set(id, source);
      if (update.kind === fault.UpdateKind.UPDATE_KIND_SOURCE) source.connected = !!update.connected;
      if (!update.key) continue;
      if (update.kind === fault.UpdateKind.UPDATE_KIND_RETIRED) source.conditions.delete(Number(update.key.condition ?? 0));
      else if (update.state) source.conditions.set(Number(update.key.condition ?? 0), update.state);
    }
  }
  view(): FaultView {
    const sources = [...this.sources.entries()];
    return {
      sources: sources.length,
      connected: sources.filter(([, source]) => source.connected).length,
      stale: this.stale,
      conditions: sources
        .flatMap(([id, source]) =>
          [...source.conditions.entries()].map(([condition, state]) => ({
            key: `${id}:${condition}`,
            source: id,
            component: state.definition?.component ?? '',
            resource: state.definition?.resource ?? '',
            code: state.definition?.code ?? '',
            severity: state.definition?.severity ?? 0,
            health: state.health ?? 0,
          })),
        )
        .filter((condition) => condition.health === Health.HEALTH_ACTIVE || condition.health === Health.HEALTH_RECOVERING)
        .sort((a, b) => b.severity - a.severity || a.key.localeCompare(b.key)),
    };
  }
}

/** Browser-side state for one run's relayed streams; reset by `configure`. */
export class LiveModel {
  private run = '';
  private signature = '';
  private kinds = new Map<string, number>();
  private sequences = new Map<string, bigint>();
  private cpu?: CpuStreams;
  private faults?: FaultStreams;
  private gaps = 0;
  /** Adopt the run's stream list; a different run or list discards received state. */
  configure(run: string, streams: Stream[]) {
    const signature = JSON.stringify(streams.map((stream) => [stream.id, stream.kind]));
    if (run === this.run && signature === this.signature) return;
    this.run = run;
    this.signature = signature;
    this.kinds = new Map(streams.map((stream) => [stream.id!, Number(stream.kind)]));
    this.sequences.clear();
    this.cpu = streams.some((stream) => stream.kind === Kind.TELEMETRY_PACKET) ? new CpuStreams() : undefined;
    this.faults = streams.some((stream) => stream.kind === Kind.FAULT_PUBLICATION) ? new FaultStreams() : undefined;
    this.gaps = 0;
  }
  /** Decode one frame; returns false for unknown streams, duplicates or malformed payloads. */
  receive(frame: Frame): boolean {
    const kind = this.kinds.get(frame.stream);
    const sequence = integer(frame.sequence);
    const previous = this.sequences.get(frame.stream) ?? 0n;
    if (kind === undefined || sequence === previous) return false;
    // A lower sequence means a restarted bridge; count skipped frames otherwise.
    if (sequence > previous + 1n && previous > 0n) this.gaps += Number(sequence - previous - 1n);
    this.sequences.set(frame.stream, sequence);
    try {
      if (kind === Kind.TELEMETRY_PACKET) this.cpu?.receive(frame.payload);
      else if (kind === Kind.FAULT_PUBLICATION) this.faults?.receive(frame.payload);
      else return false;
    } catch {
      return false;
    }
    return true;
  }
  view(): LiveView {
    return { cpu: this.cpu?.view(), faults: this.faults?.view(), gaps: this.gaps };
  }
}
