import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { readFile, rename, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { hashFile } from '../hash-file.mts';

export const artifactNames: readonly string[] = Object.freeze(readFileSync(new URL('./artifacts.txt', import.meta.url), 'utf8').trimEnd().split('\n'));
/** Parse the complete inventory; reject malformed, duplicate, missing or extra entries. */
export function parseArtifactManifest(manifest: string): Map<string, string> {
  const remaining = new Set(artifactNames), entries = new Map<string, string>(), lines = manifest.split('\n');
  if (lines.at(-1) === '') lines.pop();
  for (const line of lines) {
    const match = /^([a-f0-9]{64})  ([a-z][a-z0-9.-]*)$/i.exec(line);
    assert(match, `Malformed checksum entry: ${line}`);
    const [, digest, name] = match;
    assert(remaining.delete(name), `Unexpected or duplicate artifact: ${name}`);
    entries.set(name, digest.toLowerCase());
  }
  assert.equal(remaining.size, 0, `Missing checksum entries: ${[...remaining].join(', ')}`);
  return entries;
}
/** Require the complete artifact inventory before remote verification or extraction. */
export function verifyArtifactInventory(manifest: string): void { parseArtifactManifest(manifest); }
/** Verify all required files before installing tools or replacing caches. */
export async function verifyArtifacts(directory: string, signal?: AbortSignal): Promise<void> {
  const manifest = parseArtifactManifest(await readFile(join(directory, 'SHA256SUMS'), 'utf8'));
  for (const [name, expected] of manifest) {
    assert.equal(await hashFile(join(directory, name), signal), expected, `Artifact checksum mismatch: ${name}`);
  }
}
/** Publish a complete manifest atomically after hashing every artifact. */
export async function writeArtifactManifest(directory: string, signal?: AbortSignal): Promise<void> {
  const lines: string[] = [];
  for (const name of artifactNames) lines.push(`${await hashFile(join(directory, name), signal)}  ${name}\n`);
  const temporary = join(directory, '.SHA256SUMS.tmp');
  try {
    await writeFile(temporary, lines.join(''), { mode: 0o600, signal });
    signal?.throwIfAborted();
    await rename(temporary, join(directory, 'SHA256SUMS'));
  } finally { await rm(temporary, { force: true }); }
}
