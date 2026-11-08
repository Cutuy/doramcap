import { createWriteStream } from 'node:fs';
import { pipeline } from 'node:stream/promises';
import { createGzip } from 'node:zlib';
import { pack } from 'tar-stream';

export type ArchiveEntry = { name: string; data: Uint8Array; mode: number };

/** Stream regular-file entries into tar.gz with one timestamp, retaining exact bytes and modes.
 * Entries are consumed one at a time; input/output failures close the streams before rejection.
 */
export async function writeTarGzip(path: string, entries: Iterable<ArchiveEntry>, mtime = new Date()): Promise<void> {
  const archive = pack();
  const written = pipeline(archive, createGzip(), createWriteStream(path));
  void written.catch(() => {}); // Observe early output failures while an entry is being written.
  try {
    for (const { name, data, mode } of entries) {
      await new Promise<void>((resolve, reject) => {
        archive.entry({ name, mode, mtime, type: 'file' }, data, error => error ? reject(error) : resolve());
      });
    }
    archive.finalize();
    await written;
  } catch (error) {
    archive.destroy(error as Error);
    await written.catch(() => {});
    throw error;
  }
}
