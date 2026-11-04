import { join } from 'node:path';
import { live, wire, type App, type Config } from './native.js';
import { writeOnce } from './files.js';
import { LIMITS } from './generated/transport.js';

const RECONNECT_MS = 500;
type Stream = typeof wire.LiveStream.prototype;

/** Streams relayed for the selected apps; empty without a selected relay. Input IDs are `<app>_<index>`; app IDs exclude `_`. */
export function liveStreams(apps: App[]): Stream[] {
  if (!apps.some((app) => app.liveRelay)) return [];
  return apps.flatMap((app) =>
    (app.liveOutputs ?? []).map((output, index) =>
      wire.LiveStream.create({ id: `${app.id}_${index}`, appId: app.id!, kind: output.kind! }),
    ),
  );
}

/** Write the relay's graph fragment and its Compose overlay with the generated BridgeConfig. */
export async function bindRelay(relay: App, apps: App[], dora: Config['dora'], workspace: string, stage: string) {
  const { service, target } = relay.configMount!;
  const inputs = Object.fromEntries(
    apps.flatMap((app) =>
      (app.liveOutputs ?? []).map((output, index) => [
        `${app.id}_${index}`,
        // A slow bridge drops its oldest input and never backpressures producers.
        { source: output.source, queue_size: 4, queue_policy: 'drop_oldest' },
      ]),
    ),
  );
  const graph = join(workspace, `${stage}-${relay.id}.graph.json`);
  await writeOnce(graph, Buffer.from(JSON.stringify({ nodes: [{ id: service, path: 'dynamic', inputs }] })));
  const settings = join(workspace, `${stage}-${relay.id}.pbtxt`);
  await writeOnce(settings, Buffer.from(`port: ${relay.liveRelay!.port}\nmax_payload_bytes: ${LIMITS.bridgePayloadBytes}\n`));
  const overlay = join(workspace, `${stage}-${relay.id}-relay.json`);
  await writeOnce(overlay, Buffer.from(JSON.stringify({ services: { [service!]: {
    volumes: [{ type: 'bind', source: settings, target, read_only: true, bind: { create_host_path: false } }],
    ...(dora?.participantUid != null ? { user: `${dora.participantUid}:${dora.participantGid ?? dora.participantUid}` } : {}),
  } } })));
  return { graph, overlay };
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
