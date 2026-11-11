#!/usr/bin/env node
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createWriteStream } from 'node:fs';
import { mkdir, mkdtemp, rename, rm } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { hashFile } from './hash-file.mts';

const DOWNLOAD_TIMEOUT_MS = 120_000;
const TARGETS: Record<string, string> = { amd64: 'x86_64', arm64: 'aarch64' };
type CargoMetadata = {
  workspace_members: string[];
  packages: { id: string; name: string; version: string }[];
  resolve: { nodes: { id: string; deps: { pkg: string }[] }[] } | null;
};

/** Resolve the client's actual protocol dependency; mixed resolved protocol versions fail. */
export function doraVersion(metadata: CargoMetadata): string {
  assert(metadata.resolve, 'Resolved Cargo dependency graph required');
  const clients = metadata.packages.filter(pkg => pkg.name === 'dora-control' && metadata.workspace_members.includes(pkg.id));
  assert.equal(clients.length, 1, 'Expected one workspace dora-control client');
  const client = metadata.resolve.nodes.find(node => node.id === clients[0].id);
  assert(client, 'Dora client is absent from the resolved graph');
  const dependencies = new Set(client.deps.map(dep => dep.pkg));
  const messages = metadata.packages.filter(pkg => pkg.name === 'dora-message' && dependencies.has(pkg.id));
  assert.equal(messages.length, 1, 'Expected one resolved client dora-message dependency');
  const active = new Set(metadata.resolve.nodes.map(node => node.id));
  const versions = new Set(metadata.packages.filter(pkg => pkg.name === 'dora-message' && active.has(pkg.id)).map(pkg => pkg.version));
  assert.equal(versions.size, 1, `Mixed Dora protocol versions: ${[...versions].join(', ')}`);
  const version = messages[0].version;
  assert(/^\d+\.\d+\.\d+$/.test(version), `Expected a stable Dora protocol release: ${version}`);
  return version;
}

/** Read locked, all-feature Cargo resolution without updating dependencies. */
export function resolvedDoraVersion(repo: string): string {
  return doraVersion(JSON.parse(execFileSync('cargo', ['metadata', '--locked', '--all-features', '--format-version', '1'],
    { cwd: repo, encoding: 'utf8', maxBuffer: Infinity })));
}

function archiveName(version: string, arch: string): string {
  assert(/^\d+\.\d+\.\d+$/.test(version), `Invalid Dora release version: ${version}`);
  assert(Object.hasOwn(TARGETS, arch), `Unsupported Dora release architecture: ${arch}`);
  return `dora-cli-${TARGETS[arch]}-unknown-linux-gnu.tar.gz`;
}

/** Require the exact official release asset and its GitHub-published SHA-256 digest. */
export function doraAsset(release: any, version: string, arch: string): { url: string; sha256: string; member: string } {
  const name = archiveName(version, arch);
  assert(release && release.tag_name === `v${version}` && release.draft === false, `Expected published Dora v${version}`);
  assert(Array.isArray(release.assets), 'Dora release asset list missing');
  const matches = release.assets.filter((asset: any) => asset.name === name);
  assert.equal(matches.length, 1, `Expected one Dora v${version} asset: ${name}`);
  const asset = matches[0];
  const url = `https://github.com/dora-rs/dora/releases/download/v${version}/${name}`;
  assert.equal(asset.browser_download_url, url, 'Unexpected Dora release asset URL');
  assert.equal(asset.state, 'uploaded', 'Dora release asset is incomplete');
  assert(typeof asset.digest === 'string' && /^sha256:[a-f0-9]{64}$/.test(asset.digest), 'Dora release SHA-256 digest missing or invalid');
  return { url, sha256: asset.digest.slice('sha256:'.length), member: `${name.slice(0, -'.tar.gz'.length)}/dora` };
}

/** Check an executable against the resolved protocol release before packaging it. */
export function verifyDoraBinary(binary: string, version: string): void {
  const actual = execFileSync(resolve(binary), ['--version'], { encoding: 'utf8' }).trim();
  assert.equal(actual, `dora-cli ${version}`, `Dora binary version differs from resolved protocol ${version}`);
}

/** Download, checksum and version-check the matching native executable, then replace output atomically.
 * Missing releases/assets/digests, corrupt archives and wrong versions fail; existing output survives failure.
 */
export async function downloadDora(version: string, arch: string, output: string): Promise<void> {
  archiveName(version, arch);
  const release = await fetch(`https://api.github.com/repos/dora-rs/dora/releases/tags/v${version}`,
    { headers: { Accept: 'application/vnd.github+json' }, signal: AbortSignal.timeout(DOWNLOAD_TIMEOUT_MS) });
  assert(release.ok, `Dora v${version} release lookup failed: HTTP ${release.status}`);
  const asset = doraAsset(await release.json(), version, arch);
  output = resolve(output);
  await mkdir(dirname(output), { recursive: true });
  const stage = await mkdtemp(join(dirname(output), '.dora-'));
  try {
    const response = await fetch(asset.url, { signal: AbortSignal.timeout(DOWNLOAD_TIMEOUT_MS) });
    assert(response.ok && response.body, `Dora archive download failed: HTTP ${response.status}`);
    const archive = join(stage, 'dora.tar.gz');
    const body = response.body as import('node:stream/web').ReadableStream<Uint8Array>;
    await pipeline(Readable.fromWeb(body), createWriteStream(archive, { mode: 0o600 }));
    assert.equal(await hashFile(archive), asset.sha256, 'Dora archive checksum mismatch');
    execFileSync('tar', ['-xzf', archive, '--strip-components=1', '-C', stage, asset.member]);
    const binary = join(stage, 'dora');
    verifyDoraBinary(binary, version);
    await rename(binary, output);
  } finally { await rm(stage, { recursive: true, force: true }); }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const { values } = parseArgs({ options: { repo: { type: 'string', default: resolve(import.meta.dirname, '../..') },
      arch: { type: 'string' }, output: { type: 'string' } } });
    assert(values.arch && values.output, 'usage: dora-release.mts --arch amd64|arm64 --output FILE [--repo CHECKOUT]');
    const version = resolvedDoraVersion(values.repo);
    await downloadDora(version, values.arch, values.output);
    console.log(`Verified Dora ${version} (${values.arch}): ${resolve(values.output)}`);
  } catch (error) { console.error((error as Error).message); process.exitCode = 1; }
}
