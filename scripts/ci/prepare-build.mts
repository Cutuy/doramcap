#!/usr/bin/env node
// Bootstrap runs before npm installation; keep it dependency-free.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { lstatSync, mkdirSync, readFileSync, readdirSync, realpathSync, statSync } from 'node:fs';
import { basename, dirname, isAbsolute, join, posix, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
export const RUNNER_PATH = 'scripts/ci/wsl2/worker.mts';
export const PREPARE_PATH = 'scripts/ci/prepare-build.mts';
export type SnapshotManifest = { head?: string; dirty?: boolean; files_sha256: Record<string, string>;
  files_mode: Record<string, number>; runner_sha256: string; prepare_sha256: string };
const digest = (path: string) => createHash('sha256').update(readFileSync(path)).digest('hex');
function canonical(path: string): string {
  try { return realpathSync(path); } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    return join(canonical(dirname(resolve(path))), basename(path));
  }
}
function verify(directory: string, expected: Record<string, string>, modes: Record<string, number>, ignored: string[] = []): void {
  const actual = readdirSync(directory, { recursive: true, withFileTypes: true }).filter(entry => !entry.isDirectory()).map(entry => {
    assert(entry.isFile(), `Non-regular source file: ${entry.name}`);
    return relative(directory, join(entry.parentPath, entry.name));
  }).filter(name => !ignored.includes(name));
  assert.deepEqual(new Set(actual), new Set(Object.keys(expected)), 'Source inventory mismatch');
  for (const [name, checksum] of Object.entries(expected)) {
    assert.equal(digest(join(directory, name)), checksum, `Source checksum mismatch: ${name}`);
    if (Object.hasOwn(modes, name)) assert.equal(statSync(join(directory, name)).mode & 0o777, modes[name], `Source permission mismatch: ${name}`);
  }
}
/** Verify and synchronize a disposable cache; caller holds the worker lock through all builds/tests. */
export function prepare(snapshot: string, destination: string): void {
  const paths = [snapshot, destination].map(path => resolve(path));
  assert(paths.every(path => !lstatSync(path, { throwIfNoEntry: false })?.isSymbolicLink()), 'Snapshot and cache must be real directories');
  [snapshot, destination] = paths.map(canonical);
  assert([relative(snapshot, destination), relative(destination, snapshot)].every(path => path === '..' || path.startsWith('../')), 'Snapshot and cache must be separate, non-overlapping directories');
  const manifest: SnapshotManifest = JSON.parse(readFileSync(join(snapshot, 'SOURCE.json'), 'utf8'));
  const expected = { ...manifest.files_sha256, [RUNNER_PATH]: manifest.runner_sha256,
    [PREPARE_PATH]: manifest.prepare_sha256, 'SOURCE.json': digest(join(snapshot, 'SOURCE.json')) };
  for (const name of Object.keys(expected)) {
    assert(name && !isAbsolute(name) && !name.split('/').includes('..') && posix.normalize(name) === name, `Invalid manifest path: ${name}`);
  }
  assert.deepEqual(new Set(Object.keys(manifest.files_mode)), new Set(Object.keys(manifest.files_sha256)), 'Source permission inventory mismatch');
  verify(snapshot, expected, manifest.files_mode, ['source.tar.gz']);
  mkdirSync(destination, { recursive: true });
  execFileSync('rsync', ['-rcp', '--delete-excluded', '--exclude=/source.tar.gz', '--itemize-changes', '--', snapshot + '/', destination + '/'], { stdio: 'inherit' });
  verify(destination, expected, manifest.files_mode);
  // Keep bytecode directories CI-owned for the remaining native Python consumers.
  for (const name of Object.keys(expected).filter(name => name.endsWith('.py'))) mkdirSync(join(destination, dirname(name), '__pycache__'), { recursive: true });
  console.log(`Verified ${Object.keys(expected).length} files from ${snapshot} in ${destination}`);
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const { values, positionals } = parseArgs({ allowPositionals: true, options: { help: { type: 'boolean', short: 'h' } } });
    if (values.help) console.log('Usage: node prepare-build.mts SNAPSHOT DISPOSABLE_CACHE');
    else {
      assert.equal(positionals.length, 2, 'Expected SNAPSHOT and DISPOSABLE_CACHE');
      prepare(positionals[0], positionals[1]);
    }
  } catch (error) { console.error((error as Error).message); process.exitCode = 1; }
}
