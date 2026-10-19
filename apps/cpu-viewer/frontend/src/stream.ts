// The binary decoder preserves u64 precision and adapts integers to strings for BigInt arithmetic.
export interface CpuRecord { cgroup_id: string; cpu: string; flags: string; runtime_ns: string }
export interface TaskRecord {
  identity: { start_boottime_ns: string; cgroup_id: string; tid: string; tgid: string; cpu: string; flags: string };
  charged_runtime_ns: string;
}
export interface Snapshot {
  task_collection_enabled?: boolean;
  task_capacity?: string;
  task_records?: TaskRecord[];
  task_losses?: { dropped_events: string; dropped_runtime_ns: string; identity_errors: string };
  viewer_ingress?: {
    total_bytes: string; total_messages: string;
    bytes_per_second: number | null; messages_per_second: number | null;
    capture_to_receive_ms: number | null; mean_capture_to_receive_ms: number | null;
    invalid_timestamps: string;
  };
  sequence: string; session_id: string; source: string; capture_unix_ns: string;
  monotonic_end_ns: string; dropped_batches: string; capacity_overflow: string;
  dictionary: { id: string; path: string | null }[]; records: CpuRecord[];
}
export interface Point { time: number; duration?: number; values: Map<string, number | null> }
export const WINDOW = 30;
const MAX_POINTS = 600;
export const MAX_CPUS = 64;

export class WindowBuffer {
  private points: (Point | undefined)[] = new Array(MAX_POINTS);
  private start = 0;
  size = 0;
  prune(now: number) {
    while (this.size && this.points[this.start]!.time < now - WINDOW) {
      this.points[this.start] = undefined;
      this.start = (this.start + 1) % MAX_POINTS;
      this.size--;
    }
  }
  push(point: Point) {
    this.prune(point.time);
    if (this.size === MAX_POINTS) {
      this.points[this.start] = undefined;
      this.start = (this.start + 1) % MAX_POINTS;
      this.size--;
    }
    this.points[(this.start + this.size++) % MAX_POINTS] = point;
  }
  rows() { return Array.from({ length: this.size }, (_, i) => this.points[(this.start + i) % MAX_POINTS]!); }
}

export class CpuStream {
  buffer = new WindowBuffer();
  private previous?: { time: bigint; counters: Map<string, bigint>; arrived: number };
  private session?: string;
  private sequence?: bigint;
  private group?: string;
  gaps = 0n;
  cpus: string[] = [];
  ingest(snapshot: Snapshot, group: string, now: number) {
    const sequence = BigInt(snapshot.sequence);
    if (snapshot.session_id !== this.session) {
      this.session = snapshot.session_id;
      this.sequence = undefined;
      this.gaps = 0n;
      this.reset();
    }
    if (this.sequence !== undefined) {
      if (sequence <= this.sequence) return;
      this.gaps += sequence - this.sequence - 1n;
    }
    this.sequence = sequence;
    if (group !== this.group) { this.group = group; this.reset(); }
    const records = snapshot.records.filter(r => r.cgroup_id === group)
      .sort((a, b) => Number(a.cpu) - Number(b.cpu)).slice(0, MAX_CPUS);
    const time = BigInt(snapshot.monotonic_end_ns);
    const previous = this.previous;
    const elapsed = previous ? time - previous.time : 0n;
    const counters = new Map<string, bigint>();
    const values = new Map<string, number | null>();
    for (const record of records) {
      const flags = Number(record.flags);
      const valid = (flags & 1) !== 0 && (flags & (16 | 64)) === 0;
      const counter = BigInt(record.runtime_ns);
      const before = previous?.counters.get(record.cpu);
      let usage: number | null = null;
      if (valid) {
        counters.set(record.cpu, counter);
        if (before !== undefined && counter >= before && elapsed > 0n && now - previous!.arrived <= WINDOW) {
          usage = Number(counter - before) / Number(elapsed) * 100;
        }
      }
      values.set(record.cpu, usage);
    }
    this.previous = { time, counters, arrived: now };
    this.buffer.push({ time: now, values });
    this.cpus = [...new Set(this.buffer.rows().flatMap(p => [...p.values.keys()]))].slice(0, MAX_CPUS);
  }
  reset() { this.buffer = new WindowBuffer(); this.previous = undefined; this.cpus = []; }
  prune(now: number) {
    this.buffer.prune(now);
    if (!this.buffer.size) { this.previous = undefined; this.cpus = []; }
  }
}

export const MAX_GROUPS = 8;
export const KERNEL_SERIES = 'kernel-threads';
// Bound all-cgroup history, not just the eight currently visible series.
export const MAX_PAIRS = 8192;
export class TopCpuStreams {
  buffers = new Map<string, WindowBuffer>();
  top = new Map<string, string[]>();
  gaps = 0n;
  private previous?: { time: bigint; arrived: number; counters: Map<string, bigint> };
  private session?: string;
  private sequence?: bigint;
  private rankedAt = -Infinity;
  ingest(snapshot: Snapshot, now: number) {
    const sequence = BigInt(snapshot.sequence);
    if (snapshot.session_id !== this.session) {
      this.buffers.clear(); this.top.clear(); this.previous = undefined;
      this.sequence = undefined; this.gaps = 0n; this.rankedAt = -Infinity;
      this.session = snapshot.session_id;
    }
    if (this.sequence !== undefined) {
      if (sequence <= this.sequence) return;
      this.gaps += sequence - this.sequence - 1n;
    }
    this.sequence = sequence;
    const time = BigInt(snapshot.monotonic_end_ns);
    const previous = this.previous;
    const elapsed = previous ? time - previous.time : 0n;
    const duration = previous ? now - previous.arrived : 0;
    const counters = new Map<string, bigint>();
    const points = new Map<string, Point>();
    const cpus = new Set([...new Set(snapshot.records.map(r => r.cpu))].sort((a, b) => Number(a) - Number(b)).slice(0, MAX_CPUS));
    for (const record of snapshot.records.slice(0, MAX_PAIRS)) {
      if (!cpus.has(record.cpu)) continue;
      const key = `${record.cgroup_id}:${record.cpu}`;
      const flags = Number(record.flags);
      const valid = (flags & 1) !== 0 && (flags & (16 | 64)) === 0;
      const counter = BigInt(record.runtime_ns);
      const before = previous?.counters.get(key);
      let usage: number | null = null;
      if (valid) {
        counters.set(key, counter);
        if (before !== undefined && counter >= before && elapsed > 0n && duration > 0 && duration <= WINDOW) {
          usage = Number(counter - before) / Number(elapsed) * 100;
        }
      }
      let point = points.get(record.cpu);
      if (!point) { point = { time: now, duration, values: new Map() }; points.set(record.cpu, point); }
      point.values.set(record.cgroup_id, usage);
    }
    for (const [cpu, point] of points) {
      if (!this.buffers.has(cpu)) this.buffers.set(cpu, new WindowBuffer());
      this.buffers.get(cpu)!.push(point);
    }
    this.previous = { time, arrived: now, counters };
    this.refresh(now);
  }
  refresh(now: number) {
    for (const [cpu, buffer] of this.buffers) {
      buffer.prune(now);
      if (!buffer.size) { this.buffers.delete(cpu); this.top.delete(cpu); }
    }
    if (!this.buffers.size) this.previous = undefined;
    if (now - this.rankedAt < 1) return;
    this.rankedAt = now;
    for (const [cpu, buffer] of this.buffers) {
      const scores = new Map<string, number>();
      for (const point of buffer.rows()) {
        // Time-weighted contribution over the last five seconds. Missing,
        // reset and unavailable counters never enter the ranking.
        const weight = Math.max(0, point.time - Math.max(now - 5, point.time - (point.duration ?? 0)));
        if (!weight) continue;
        for (const [id, value] of point.values) {
          if (id !== KERNEL_SERIES && value !== null && value > 0) scores.set(id, (scores.get(id) ?? 0) + value * weight);
        }
      }
      this.top.set(cpu, [...scores].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).slice(0, MAX_GROUPS).map(([id]) => id));
    }
  }
}
