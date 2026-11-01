import { viewer, wire, type ConsoleBackend } from './native.js';
const TELEMETRY_VERSION = 2;
const MAX_FRAME_BYTES = 4 * 1024 * 1024;
const MAX_RETAINED_FRAME_BYTES = 512 * 1024;
export const MAX_SNAPSHOTS = 2;
export const MAX_GROUP_NAMES = 512;
const MAX_GROUP_NAME_CHARS = 128;
const MAX_SAMPLE_AGE_MS = 500;
const integer = (value: unknown) => BigInt(String(value ?? 0));
const options = { longs: String, enums: String, defaults: true } as const;
type Status = Record<string, any>;

/** Consume the existing CPU viewer feed; retain two snapshots and sanitized leaf names. */
export class CpuFeeds {
  private socket?: WebSocket;
  private timer?: ReturnType<typeof setInterval>;
  private run = '';
  private endpoint = '';
  private snapshots: any[] = [];
  private dictionary: any;
  private incomplete = false;
  private selected = false;
  private receivedAt = 0;
  constructor(
    private native: Pick<ConsoleBackend, 'status'>,
    private feeds: { appId?: string | null; url?: string | null }[],
  ) {
    if (feeds.length) {
      this.timer = setInterval(() => void this.poll(), 500);
      this.timer.unref();
    }
  }
  private async poll() {
    try {
      this.enrich(wire.Status.toObject(wire.Status.decode(await this.native.status()), options));
    } catch {
      this.disconnect();
    }
  }
  private disconnect() {
    this.socket?.close();
    this.socket = undefined;
  }
  close() {
    clearInterval(this.timer);
    this.disconnect();
  }
  enrich(status: Status): Status {
    if (status.runId !== this.run) {
      this.disconnect();
      this.run = status.runId ?? '';
      this.endpoint = '';
      this.snapshots = [];
      this.dictionary = undefined;
      this.incomplete = false;
    }
    const active = ['STARTING', 'RUNNING', 'STOPPING'].includes(status.phase);
    const selected = this.feeds.find((feed) => status.systemApps?.ids?.includes(feed.appId));
    this.selected = !!selected;
    status = { ...status, cpuCaptureEnabled: this.selected };
    if (status.phase === 'RUNNING' && selected?.url && !this.socket) {
      this.endpoint = selected.url;
      const socket = new WebSocket(this.endpoint);
      socket.binaryType = 'arraybuffer';
      this.socket = socket;
      socket.onmessage = (event) => {
        try {
          this.observe(new Uint8Array(event.data as ArrayBuffer));
        } catch {
          this.incomplete = true;
          socket.close();
        }
      };
      socket.onclose = () => {
        if (this.socket === socket) this.socket = undefined;
      };
      socket.onerror = () => {
        this.incomplete = true;
        socket.close();
      };
    } else if (!active || !selected) {
      this.disconnect();
    }
    if (!selected) return status;
    if (!active && !this.snapshots.length)
      return {
        ...status,
        cpuSnapshots: status.cpuSnapshots ?? [],
        cpuGroupNames: status.cpuGroupNames ?? [],
      };
    const snapshots = this.snapshots.map((frame) => frame.snapshot);
    const last = snapshots.at(-1);
    const parents = new Set((last?.records ?? []).map((record: any) => String(record.parentId)));
    const leaves = new Set(
      (last?.records ?? [])
        .filter((record: any) => !parents.has(String(record.cgroupId)))
        .map((record: any) => String(record.cgroupId)),
    );
    const names = (this.dictionary?.entries ?? [])
      .filter((entry: any) => leaves.has(String(entry.id)))
      .slice(0, MAX_GROUP_NAMES)
      .map((entry: any) => ({
        id: String(entry.id),
        name:
          String(entry.path ?? '')
            .split('/')
            .filter(Boolean)
            .at(-1)
            ?.slice(0, MAX_GROUP_NAME_CHARS) ?? '',
      }))
      .filter((entry: any) => entry.name && entry.name !== '.' && entry.name !== '..');
    return {
      ...status,
      cpuSnapshots: snapshots,
      cpuGroupNames: names,
      monitorReady: !!last && !!this.dictionary,
      metricsIncomplete: !!status.metricsIncomplete || this.incomplete,
    };
  }
  /** Derive core percent exclusively from the selected monitor's cumulative host counters. */
  sample(run: string, now = performance.now()) {
    if (!this.selected || run !== this.run) return undefined;
    const [before, after] = this.snapshots;
    const current = after?.snapshot?.cpuTotals ?? before?.snapshot?.cpuTotals;
    const unavailable = { cpuPercent: null as number | null, cpuCores: current?.cpuCores ?? 0 };
    if (!before || !after || now - this.receivedAt > MAX_SAMPLE_AGE_MS || before.sessionId !== after.sessionId) return unavailable;
    const old = before.snapshot?.cpuTotals, next = after.snapshot?.cpuTotals;
    if (!old || !next || !next.cpuCores || old.cpuCores !== next.cpuCores || integer(after.snapshot.sequence) <= integer(before.snapshot.sequence)) return unavailable;
    const total = integer(next.totalTicks) - integer(old.totalTicks);
    const busy = integer(next.busyTicks) - integer(old.busyTicks);
    if (total <= 0n || busy < 0n || busy > total) return unavailable;
    return { cpuPercent: Number(busy) / Number(total) * next.cpuCores * 100, cpuCores: next.cpuCores };
  }
  observe(bytes: Uint8Array) {
    if (bytes.byteLength > MAX_FRAME_BYTES) throw new Error('CPU feed frame too large');
    if (bytes.byteLength > MAX_RETAINED_FRAME_BYTES) {
      this.incomplete = true;
      return;
    }
    const packet = viewer.Packet.toObject(viewer.Packet.decode(bytes), options);
    if (packet.dictionary) {
      if (packet.dictionary.version !== TELEMETRY_VERSION) throw new Error('invalid CPU dictionary');
      this.dictionary = packet.dictionary;
      return;
    }
    const frame = packet.snapshot;
    if (!frame?.snapshot || frame.version !== TELEMETRY_VERSION) throw new Error('invalid CPU feed packet');
    if (this.snapshots.at(-1)?.snapshot?.sequence !== frame.snapshot.sequence) {
      this.snapshots.push(frame);
      this.receivedAt = performance.now();
    }
    this.snapshots = this.snapshots.slice(-MAX_SNAPSHOTS);
  }
}
