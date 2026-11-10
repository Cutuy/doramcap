#!/usr/bin/env node
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { createWriteStream, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { fileURLToPath } from 'node:url';
import { Command } from 'commander';
import { execaSync } from 'execa';
import { hashFile } from './hash-file.mts';
import { sshOptions, workerHost, workerSsh } from './worker-ssh.mts';

const NODE_DISTRIBUTION = 'https://nodejs.org/dist';
const DOWNLOAD_TIMEOUT_MS = 120_000;
/** Resolve exactly one published checksum for the requested release archive. */
export function nodeArchiveChecksum(manifest: string, archive: string): string {
  const entries = manifest.split('\n').map(line => /^([a-f0-9]{64})  (.+)$/.exec(line)).filter(match => match?.[2] === archive);
  assert.equal(entries.length, 1, `Expected one Node checksum for ${archive}`);
  return entries[0]![1];
}
/** Download and verify a pinned ARM64 Node archive; base URL is injectable for fixture tests. */
export async function downloadNodeArchive(version: string, destination: string, baseURL = NODE_DISTRIBUTION): Promise<string> {
  assert(/^\d+\.\d+\.\d+$/.test(version), `Invalid Node version: ${version}`);
  const archive = `node-v${version}-linux-arm64.tar.gz`, base = `${baseURL}/v${version}`;
  const checksums = await fetch(`${base}/SHASUMS256.txt`, { signal: AbortSignal.timeout(DOWNLOAD_TIMEOUT_MS) });
  assert(checksums.ok, `Node checksum download failed: HTTP ${checksums.status}`);
  const expected = nodeArchiveChecksum(await checksums.text(), archive);
  const response = await fetch(`${base}/${archive}`, { signal: AbortSignal.timeout(DOWNLOAD_TIMEOUT_MS) });
  assert(response.ok && response.body, `Node archive download failed: HTTP ${response.status}`);
  const body = response.body as import('node:stream/web').ReadableStream<Uint8Array>;
  await pipeline(Readable.fromWeb(body), createWriteStream(destination, { mode: 0o600 }));
  assert.equal(await hashFile(destination), expected, 'Node archive checksum mismatch');
  return expected;
}
/** Provision the repository-pinned Node runtime in the CI account, preserving system Node. */
export async function installWorkerNode(host: string): Promise<void> {
  host = workerHost(host);
  const version = readFileSync(new URL('../../.node-version', import.meta.url), 'utf8').trim();
  assert(/^\d+\.\d+\.\d+$/.test(version), `Invalid pinned Node version: ${version}`);
  assert.equal(workerSsh(host, 'uname -sm').trim(), 'Linux aarch64', 'Node worker must be Linux aarch64');
  if (workerSsh(host, 'if test -x "$HOME/.local/bin/node"; then "$HOME/.local/bin/node" --version; fi').trim() === `v${version}`) {
    console.log(`${host}: Node v${version} already installed`); return;
  }
  const runtime = `node-v${version}-linux-arm64`, target = `$HOME/.local/lib/${runtime}`;
  const activate = ['mkdir -p "$HOME/.local/bin"', ...['node', 'npm', 'npx'].map(binary =>
    `ln -sfn "${target}/bin/${binary}" "$HOME/.local/bin/${binary}"`)].join(' && ');
  if (workerSsh(host, `if test -x "${target}/bin/node"; then "${target}/bin/node" --version; fi`).trim() === `v${version}`) {
    workerSsh(host, activate);
    console.log(`${host}: activated Node v${version}`); return;
  }
  for (const tool of ['tar', 'gzip', 'sha256sum']) {
    assert(workerSsh(host, `command -v ${tool} || true`).trim(), `Missing Node installation tool on ${host}: ${tool}`);
  }
  const local = mkdtempSync(join(tmpdir(), 'doramcap-node-')), stage = `.cache/doramcap-node/${randomUUID().replaceAll('-', '')}`;
  let remoteStaged = false;
  try {
    const archive = join(local, 'node.tar.gz'), checksum = await downloadNodeArchive(version, archive);
    workerSsh(host, `umask 077; mkdir -p ${stage}`);
    remoteStaged = true;
    execaSync('scp', [...sshOptions, archive, `${host}:${stage}/node.tar.gz`], { stdio: 'inherit' });
    workerSsh(host, `cd ${stage} && echo '${checksum}  node.tar.gz' | sha256sum -c - && tar -xzf node.tar.gz && test "$(./${runtime}/bin/node --version)" = v${version} && mkdir -p "$HOME/.local/lib" && test ! -e "${target}" && mv ${runtime} "${target}" && ${activate}`);
    assert.equal(workerSsh(host, '"$HOME/.local/bin/node" --version').trim(), `v${version}`);
    console.log(`${host}: installed Node v${version}`);
  } finally {
    rmSync(local, { recursive: true, force: true });
    if (remoteStaged) {
      try { workerSsh(host, `rm -rf ${stage}`); }
      catch (error) { console.error(`Node staging cleanup failed at ${host}:${stage}: ${(error as Error).message}`); }
    }
  }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = new Command().requiredOption('--host <host>', 'ARM64 SSH worker').parse().opts();
  try { await installWorkerNode(args.host); }
  catch (error) { console.error((error as Error).message); process.exitCode = 1; }
}
