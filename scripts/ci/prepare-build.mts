#!/usr/bin/env node
// Verify a submitted snapshot and synchronize a dedicated, disposable source cache.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { lstatSync, mkdirSync, readFileSync, readdirSync, realpathSync, statSync } from 'node:fs';
import { basename, dirname, isAbsolute, join, posix, resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';
import { isDeepStrictEqual, parseArgs } from 'node:util';

export type SnapshotManifest = {
  head?: string; dirty?: boolean;
  files_sha256: Record<string, string>; files_mode: Record<string, number>;
  runner_sha256: string; prepare_sha256: string;
};

/** Return the SHA256 of a regular file's bytes. */
export function digest(path: string): string {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function files(directory: string, ignored: string[], prefix = ''): Set<string> {
  const result = new Set<string>();
  for (const entry of readdirSync(join(directory, prefix), { withFileTypes: true })) {
    const relative = posix.join(prefix, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`Symlink in source tree: ${relative}`);
    if (entry.isDirectory()) {
      for (const name of files(directory, ignored, relative)) result.add(name);
    } else {
      if (!entry.isFile()) throw new Error(`Non-regular source file: ${relative}`);
      if (!ignored.includes(relative)) result.add(relative);
    }
  }
  return result;
}

function verify(directory: string, expected: Record<string, string>, modes: Record<string, number>, ignored: string[] = []): void {
  const actual = files(directory, ignored);
  const names = new Set(Object.keys(expected));
  if (!isDeepStrictEqual(actual, names)) {
    throw new Error(`Source inventory mismatch: missing=${[...names].filter(name => !actual.has(name))}, extra=${[...actual].filter(name => !names.has(name))}`);
  }
  for (const [name, checksum] of Object.entries(expected)) {
    if (digest(join(directory, name)) !== checksum) throw new Error(`Source checksum mismatch: ${name}`);
    if (Object.hasOwn(modes, name) && (statSync(join(directory, name)).mode & 0o777) !== modes[name]) {
      throw new Error(`Source permission mismatch: ${name}`);
    }
  }
}

function canonical(path: string): string {
  try { return realpathSync(path); }
  catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    return join(canonical(dirname(path)), basename(path));
  }
}

/** Synchronize verified inputs with rsync; preserve identical mtimes and remove stale cache files.
 * Caller holds the worker lock through synchronization and all builds/tests. Invalid inputs fail before mutation.
 */
export function prepare(snapshot: string, destination: string): void {
  snapshot = resolve(snapshot);
  destination = resolve(destination);
  if (lstatSync(snapshot, { throwIfNoEntry: false })?.isSymbolicLink()
      || lstatSync(destination, { throwIfNoEntry: false })?.isSymbolicLink()) {
    throw new Error('Snapshot and cache must be real directories');
  }
  snapshot = canonical(resolve(snapshot));
  destination = canonical(resolve(destination));
  const contains = (parent: string, child: string) => child.startsWith(parent.endsWith(sep) ? parent : parent + sep);
  if (snapshot === destination || contains(destination, snapshot) || contains(snapshot, destination)) {
    throw new Error('Snapshot and cache must be separate, non-overlapping directories');
  }
  const manifest: SnapshotManifest = JSON.parse(readFileSync(join(snapshot, 'SOURCE.json'), 'utf8'));
  const expected = { ...manifest.files_sha256,
    'run-worker-job.sh': manifest.runner_sha256, 'prepare-build.mts': manifest.prepare_sha256,
    'SOURCE.json': digest(join(snapshot, 'SOURCE.json')),
  };
  for (const name of Object.keys(expected)) {
    if (!name || isAbsolute(name) || name.split('/').includes('..') || posix.normalize(name) !== name) {
      throw new Error(`Invalid manifest path: ${name}`);
    }
  }
  const modes = manifest.files_mode;
  if (!isDeepStrictEqual(new Set(Object.keys(modes)), new Set(Object.keys(manifest.files_sha256)))) {
    throw new Error('Source permission inventory mismatch');
  }
  verify(snapshot, expected, modes, ['source.tar.gz']);
  mkdirSync(destination, { recursive: true });
  // rsync replaces stale cache symlinks without following their targets.
  execFileSync('rsync', ['-rcp', '--delete-excluded', '--exclude=/source.tar.gz',
    '--itemize-changes', '--', snapshot + '/', destination + '/'], { stdio: 'inherit' });
  verify(destination, expected, modes);
  // Keep bytecode directories CI-owned for remaining native Python consumers.
  for (const name of Object.keys(expected)) {
    if (name.endsWith('.py')) mkdirSync(join(destination, dirname(name), '__pycache__'), { recursive: true });
  }
  console.log(`Verified ${Object.keys(expected).length} files from ${snapshot} in ${destination}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    const { values, positionals } = parseArgs({ allowPositionals: true, options: { help: { type: 'boolean', short: 'h' } } });
    if (values.help) console.log('Usage: node prepare-build.mts SNAPSHOT DISPOSABLE_CACHE');
    else {
      if (positionals.length !== 2) throw new Error('Expected SNAPSHOT and DISPOSABLE_CACHE');
      prepare(positionals[0], positionals[1]);
    }
  } catch (error) { console.error((error as Error).message); process.exitCode = 1; }
}
