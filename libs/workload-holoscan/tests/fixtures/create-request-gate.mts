/** Transparent test proxy: hold one real monitor create before daemon acceptance; record client close. */
import { strict as assert } from 'node:assert';
import { createServer, request } from 'node:http';
import type { Socket } from 'node:net';

export async function createRequestGate(socket: string, daemon: string) {
  let unlock!: () => void, arrived!: (labels: Record<string, string>) => void, closed!: () => void;
  const release = new Promise<void>(resolve => unlock = resolve);
  const reached = new Promise<Record<string, string>>(resolve => arrived = resolve);
  const clientClosed = new Promise<void>(resolve => closed = resolve);
  let held = false, forwarded = false, cancelledBeforeForward = false;
  const sockets = new Set<Socket>();
  const server = createServer(async (incoming, response) => {
    try {
      // Buffer only JSON create bodies; every other request/response stays streaming.
      const create = incoming.method === 'POST' && incoming.url?.includes('/containers/create');
      const chunks: Buffer[] = [];
      if (create) for await (const chunk of incoming) chunks.push(Buffer.from(chunk));
      const bytes = Buffer.concat(chunks);
      const labels = create ? JSON.parse(bytes.toString()).Labels ?? {} : {};
      const gate = create && !held && labels['com.docker.compose.service'] === 'cpu-monitor';
      if (gate) {
        held = true;
        response.once('close', () => { if (!response.writableEnded) closed(); });
        arrived(labels);
        await release;
        // Observe cancellation explicitly. This says nothing about a request accepted by Docker.
        if (response.destroyed || incoming.socket.destroyed) { cancelledBeforeForward = true; return; }
        forwarded = true;
      }
      const upstream = request({ socketPath: daemon, path: incoming.url, method: incoming.method, headers: incoming.headers }, result => {
        response.writeHead(result.statusCode ?? 500, result.headers);
        result.pipe(response); result.on('error', error => response.destroy(error));
      });
      upstream.on('error', error => response.destroy(error));
      response.once('close', () => upstream.destroy());
      if (create) upstream.end(bytes); else incoming.pipe(upstream);
    } catch (error) { response.destroy(error as Error); }
  });
  server.on('connection', socket => { sockets.add(socket); socket.once('close', () => sockets.delete(socket)); });
  await new Promise<void>((resolve, reject) => { server.once('error', reject); server.listen(socket, resolve); });
  return {
    reached, clientClosed, release() { unlock(); },
    observation() { return { held, forwarded, cancelledBeforeForward }; },
    async close() { unlock(); for (const socket of sockets) socket.destroy(); await new Promise<void>(resolve => server.close(() => resolve())); },
    assertCancelled() { assert.deepEqual({ forwarded, cancelledBeforeForward }, { forwarded: false, cancelledBeforeForward: true }); },
  };
}
