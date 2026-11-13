import { createReadStream } from 'node:fs';
import type { ServerResponse } from 'node:http';
import { pipeline, Readable } from 'node:stream';
import { artifactFilename, displayPath, FILE_CHANGES_ARTIFACT, RUN_FILE_ARTIFACT } from './generated/transport.js';
import type { doramcap } from './generated/console.js';
import type { ConsoleBackend } from './native.js';

/** Stream an already-authorized descriptor; pipeline closes it on disconnect. */
export async function streamArtifact(native: ConsoleBackend, response: ServerResponse, runId: string, kind: string, request?: doramcap.console.v1.IRunFileRequest): Promise<void> {
  if (kind === FILE_CHANGES_ARTIFACT) {
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
        'content-disposition': attachment(Buffer.from(artifactFilename(kind))),
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
  const controller = new AbortController();
  const abort = () => controller.abort();
  response.once('close', abort);
  let source: ReturnType<typeof createReadStream> | undefined;
  try {
    const opened = kind === RUN_FILE_ARTIFACT
      ? await native.openRunFile(runId, request!, controller.signal)
      : { fd: await native.openArtifact(runId, kind), filename: Buffer.from(artifactFilename(kind)) };
    source = createReadStream('', { fd: opened.fd, autoClose: true });
    if (response.destroyed) { source.destroy(); return; }
    response.writeHead(200, {
      'content-type': kind === 'report' ? 'application/json' : 'application/octet-stream',
      'content-disposition': attachment(opened.filename),
      'cache-control': 'no-store',
      'x-content-type-options': 'nosniff',
    });
    await new Promise<void>((resolve, reject) => {
      pipeline(source!, response, error => error ? reject(error) : resolve());
    });
  } catch (error) {
    source?.destroy();
    throw error;
  } finally {
    controller.abort();
    response.off('close', abort);
  }
}

// Browsers use Unicode names; invalid Unix bytes keep their escaped display spelling.
function attachment(bytes: Uint8Array) {
  const name = displayPath(bytes);
  const fallback = /^[A-Za-z0-9._ -]+$/.test(name) ? name : 'download';
  const encoded = encodeURIComponent(name).replace(/[!'()*]/g, char => `%${char.charCodeAt(0).toString(16).toUpperCase()}`);
  return `attachment; filename="${fallback}"; filename*=UTF-8''${encoded}`;
}
