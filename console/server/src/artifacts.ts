import { createReadStream } from 'node:fs';
import type { ServerResponse } from 'node:http';
import { pipeline } from 'node:stream';
import type { ConsoleBackend } from './native.js';

/** Stream an already-authorized descriptor; pipeline closes it on disconnect. */
export async function streamArtifact(native: ConsoleBackend, response: ServerResponse, runId: string, kind: string): Promise<void> {
  const fd = await native.openArtifact(runId, kind);
  const source = createReadStream('', { fd, autoClose: true });
  if (response.destroyed) { source.destroy(); return; }
  try {
    response.writeHead(200, {
      'content-type': kind === 'report' ? 'application/json' : 'application/octet-stream',
      'content-disposition': `attachment; filename="${kind === 'report' ? 'console-report.json' : `${kind}.mcap`}"`,
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
