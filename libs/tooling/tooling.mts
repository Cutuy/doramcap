import { Command, CommanderError } from 'commander';
import { execaSync } from 'execa';
import { Minimatch } from 'minimatch';
import { pack } from 'tar-stream';
import { createHash } from 'node:crypto';
import { createWriteStream, mkdtempSync, readFileSync, realpathSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { pipeline } from 'node:stream/promises';
import { pathToFileURL } from 'node:url';
import { createGzip } from 'node:zlib';

export { Command };

/** Identify direct execution while keeping an imported command module side-effect free. */
export const isMain = (url: string): boolean => Boolean(process.argv[1] && url === pathToFileURL(resolve(process.argv[1])).href);

/** Parse a Commander CLI; report failures without exiting before resource cleanup finishes. */
export async function cli(command: Command, failureCode = 1): Promise<void> {
  try { await command.exitOverride().parseAsync(); }
  catch (error) {
    if (!(error instanceof CommanderError)) console.error((error as Error).message);
    process.exitCode = error instanceof CommanderError && error.exitCode === 0 ? 0 : failureCode;
  }
}

/** Run argv directly with inherited output; nonzero exits, signals and spawn failures throw ExecaError. */
export const run = execaSync({ stdio: 'inherit' });
/** Capture complete UTF-8 output, preserving trailing newlines; failures throw ExecaError. */
export const capture = execaSync({ stripFinalNewline: false, maxBuffer: Infinity });

/** Hash the supplied bytes as lowercase SHA256 hex. */
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

/** Compile case-sensitive minimatch patterns, including dotfiles; match when any pattern accepts the name. */
export function globMatcher(patterns: readonly string[]): (name: string) => boolean {
  const matchers = patterns.map(pattern => new Minimatch(pattern, { dot: true }));
  return name => matchers.some(matcher => matcher.match(name));
}

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
