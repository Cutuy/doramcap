// Bootstrap-safe filesystem helpers use Node built-ins only.
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, realpathSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';

/** Hash the exact supplied bytes as lowercase SHA256 hex. */
export const sha256 = (data: Uint8Array): string => createHash('sha256').update(data).digest('hex');
/** Hash a file's bytes as lowercase SHA256 hex. */
export const digest = (path: string): string => sha256(readFileSync(path));

/** Resolve existing symlinks and retain missing path components; other filesystem errors propagate. */
export function canonicalPath(path: string): string {
  path = resolve(path);
  try { return realpathSync(path); }
  catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    return join(canonicalPath(dirname(path)), basename(path));
  }
}

/** Own a temporary directory until the callback settles; remove it on success or failure. */
export async function withTempDirectory<T>(prefix: string, action: (path: string) => T | Promise<T>): Promise<T> {
  const directory = mkdtempSync(join(tmpdir(), prefix));
  try { return await action(directory); }
  finally { rmSync(directory, { recursive: true, force: true }); }
}
