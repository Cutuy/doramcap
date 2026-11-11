import { createReadStream } from 'node:fs';
import type { ServerResponse } from 'node:http';
import { pipeline, Readable } from 'node:stream';
import { artifactFilename } from './generated/transport.js';
import type { ConsoleBackend } from './native.js';

/** Stream an already-authorized descriptor; pipeline closes it on disconnect. */
export async function streamArtifact(native: ConsoleBackend, response: ServerResponse, runId: string, kind: string): Promise<void> {
  if (kind === 'file-changes') {
    const controller = new AbortController();
    const abort = () => controller.abort();
    response.once('close', abort);
    const iterator = native.fileChanges(runId, controller.signal)[Symbol.asyncIterator]();
    let source: Readable | undefined;
    try {
      if (response.destroyed) return;
      // Prime authorization/checkpoint errors before sending a successful response.
      const first = await iterator.next();
      if (response.destroyed) return;
      response.writeHead(200, {
        'content-type': 'application/x-ndjson',
        'content-disposition': `attachment; filename="${artifactFilename(kind)}"`,
        'cache-control': 'no-store', 'x-content-type-options': 'nosniff',
      });
      source = Readable.from((async function* () {
        if (!first.done) yield first.value;
        for await (const chunk of { [Symbol.asyncIterator]: () => iterator }) yield chunk;
      })());
      await new Promise<void>((resolve, reject) => pipeline(source!, response, error => error ? reject(error) : resolve()));
    } finally {
      controller.abort(); source?.destroy();
      await iterator.return?.();
      response.off('close', abort);
    }
    return;
  }
  const fd = await native.openArtifact(runId, kind);
  const source = createReadStream('', { fd, autoClose: true });
  if (response.destroyed) { source.destroy(); return; }
  try {
    response.writeHead(200, {
      'content-type': kind === 'report' ? 'application/json' : 'application/octet-stream',
      'content-disposition': `attachment; filename="${artifactFilename(kind)}"`,
      'cache-control': 'no-store',
      'x-content-type-options': 'nosniff',
    });
    await new Promise<void>((resolve, reject) => {
      pipeline(source, response, error => error ? reject(error) : resolve());
    });
  } catch (error) {
    source.destroy();
    throw error;
  }
}
