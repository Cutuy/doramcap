import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat } from 'node:fs/promises';

/** Stream a regular file through SHA256 with bounded memory and cancellation. */
export async function hashFile(path: string, signal?: AbortSignal): Promise<string> {
  signal?.throwIfAborted();
  assert((await lstat(path)).isFile(), `Artifact must be a regular file: ${path}`);
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path, { signal })) hash.update(chunk);
  signal?.throwIfAborted();
  return hash.digest('hex');
}
