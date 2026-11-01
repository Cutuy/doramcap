import { join } from 'node:path';
import { live, wire, type App, type Config } from './native.js';
import { writeOnce } from './store.js';
import { LIMITS } from './generated/transport.js';

/** Optional system app listed beside the Dora services. */
export const LIVE_BRIDGE = { id: 'live-bridge', label: 'Live telemetry' } as const;
const RECONNECT_MS = 500;
type Stream = typeof wire.LiveStream.prototype;

/** Bridge input IDs are `<app>_<index>`; app IDs exclude `_`, so IDs are unique. */
export function liveStreams(apps: App[]): Stream[] {
  return apps.flatMap((app) =>
    (app.liveOutputs ?? []).map((output, index) =>
      wire.LiveStream.create({ id: `${app.id}_${index}`, appId: app.id!, kind: output.kind! }),
    ),
  );
}

/** Write the bridge's graph fragment and Compose project for the selected apps' outputs. */
export async function bindBridge(config: Config, apps: App[], workspace: string, stage: string) {
  const bridge = config.liveBridge!;
  const dora = config.dora;
  const inputs = Object.fromEntries(
    apps.flatMap((app) =>
      (app.liveOutputs ?? []).map((output, index) => [
        `${app.id}_${index}`,
        // A slow bridge drops its oldest input and never backpressures producers.
        { source: output.source, queue_size: 4, queue_policy: 'drop_oldest' },
      ]),
    ),
  );
  const graph = join(workspace, `${stage}-${LIVE_BRIDGE.id}.graph.json`);
  await writeOnce(graph, Buffer.from(JSON.stringify({ nodes: [{ id: LIVE_BRIDGE.id, path: 'dynamic', inputs }] })));
  const ready = `/tmp/${LIVE_BRIDGE.id}.ready`;
  const service = {
    image: bridge.image,
    entrypoint: ['/usr/local/bin/live-bridge'],
    command: ['--port', String(bridge.port), '--max-payload-bytes', String(LIMITS.bridgePayloadBytes)],
    ...(dora?.participantUid != null ? { user: `${dora.participantUid}:${dora.participantGid ?? dora.participantUid}` } : {}),
    network_mode: 'host',
    init: true,
    read_only: true,
    cap_drop: ['ALL'],
    security_opt: ['no-new-privileges:true'],
    tmpfs: ['/tmp'],
    stop_signal: 'SIGINT',
    stop_grace_period: '15s',
    environment: { DORAMCAP_READY_FILE: ready },
    healthcheck: { test: ['CMD', 'test', '-s', ready], interval: '200ms', timeout: '1s', retries: 150 },
  };
  const file = join(workspace, `${stage}-${LIVE_BRIDGE.id}.json`);
  await writeOnce(file, Buffer.from(JSON.stringify({ services: { [LIVE_BRIDGE.id]: service } })));
  return { graph, project: { key: LIVE_BRIDGE.id, directory: workspace, files: [file] } };
}

/**
 * One bridge WebSocket per running run; frames fan out to browser listeners.
 * Only the `Frame` envelope is decoded; the latest frame per stream replays to new listeners.
 */
export class LiveRelay {
  private socket?: WebSocket;
  private retry?: ReturnType<typeof setTimeout>;
  private run = '';
  private latest = new Map<string, Buffer>();
  private listeners = new Set<(frame: Buffer) => void>();
  constructor(private url: string) {}
  /** Connect for `run`; frames from an earlier run are discarded. */
  open(run: string) {
    if (run !== this.run) {
      this.close();
      this.run = run;
    }
    if (!this.socket && !this.retry) this.connect();
  }
  close() {
    clearTimeout(this.retry);
    this.retry = undefined;
    const socket = this.socket;
    this.socket = undefined;
    socket?.close();
    this.run = '';
    this.latest.clear();
  }
  /** Replay latest frames to `consume`, then deliver live frames until unsubscribed. */
  subscribe(consume: (frame: Buffer) => void): () => void {
    for (const frame of this.latest.values()) consume(frame);
    this.listeners.add(consume);
    return () => this.listeners.delete(consume);
  }
  /** Retain and fan out one encoded `Frame`; false rejects a malformed or oversized frame. */
  receive(frame: Buffer): boolean {
    if (!frame.length || frame.length > LIMITS.frameBytes) return false;
    let decoded: InstanceType<typeof live.Frame>;
    try {
      decoded = live.Frame.decode(frame);
    } catch {
      return false;
    }
    const stream = decoded.stream;
    if (!stream || decoded.payload.length > LIMITS.bridgePayloadBytes) return false;
    this.latest.set(stream, frame);
    for (const listener of this.listeners) listener(frame);
    return true;
  }
  private connect() {
    const socket = new WebSocket(this.url);
    socket.binaryType = 'arraybuffer';
    this.socket = socket;
    socket.onmessage = (event) => {
      if (this.socket === socket && !this.receive(Buffer.from(event.data as ArrayBuffer))) socket.close();
    };
    socket.onerror = () => socket.close();
    socket.onclose = () => {
      if (this.socket !== socket) return;
      this.socket = undefined;
      this.retry = setTimeout(() => {
        this.retry = undefined;
        if (this.run) this.connect();
      }, RECONNECT_MS);
      this.retry.unref();
    };
  }
}
