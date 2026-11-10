import assert from 'node:assert/strict';
import { chmod, copyFile, mkdir, mkdtemp, rename, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createWriteStream } from 'node:fs';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { setTimeout as delay } from 'node:timers/promises';
import { hashFile } from './worker-artifacts.mts';
import type { WorkerCommand } from './worker-process.mts';

const VERSION = '0.9.151';
const ARCHIVE_SHA256 = '3e35e8f6913af92558ba09d7155f74e35dd1f0024ef3cd891637433ae4eacbdd';
const DOWNLOAD_TIMEOUT_MS = 120_000, DOWNLOAD_ATTEMPTS = 4, RETRY_DELAY_MS = 2_000;
/** Install the pinned nextest release atomically; existing matching versions are reused. */
export async function installNextest(bin: string, command: WorkerCommand, signal: AbortSignal): Promise<void> {
  const target = join(bin, 'cargo-nextest');
  try {
    const current = await command(target, ['--version'], { capture: true });
    if (current.code === 0 && current.stdout.startsWith(`cargo-nextest ${VERSION} `)) return;
  } catch (error) {
    if (signal.aborted || (error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
  }
  const stage = await mkdtemp(join(tmpdir(), 'doramcap-nextest-'));
  const temporary = join(bin, '.cargo-nextest.installing');
  try {
    const archive = join(stage, 'nextest.tar.gz');
    for (let attempt = 0; ; attempt++) {
      signal.throwIfAborted();
      try {
        const downloadSignal = AbortSignal.any([signal, AbortSignal.timeout(DOWNLOAD_TIMEOUT_MS)]);
        const response = await fetch(`https://get.nexte.st/${VERSION}/linux-arm`, { signal: downloadSignal });
        assert(response.ok && response.body, `Nextest download failed: HTTP ${response.status}`);
        const body = response.body as import('node:stream/web').ReadableStream<Uint8Array>;
        await pipeline(Readable.fromWeb(body), createWriteStream(archive, { mode: 0o600 }));
        break;
      } catch (error) {
        if (signal.aborted || attempt + 1 === DOWNLOAD_ATTEMPTS) throw error;
        await delay(RETRY_DELAY_MS, undefined, { signal });
      }
    }
    assert.equal(await hashFile(archive, signal), ARCHIVE_SHA256, 'Nextest release checksum mismatch');
    const unpacked = await command('tar', ['-xzf', archive, '-C', stage]);
    assert.equal(unpacked.code, 0, 'Nextest extraction failed');
    await mkdir(bin, { recursive: true });
    await copyFile(join(stage, 'cargo-nextest'), temporary); await chmod(temporary, 0o755);
    signal.throwIfAborted(); await rename(temporary, target);
  } finally { await rm(temporary, { force: true }); await rm(stage, { recursive: true, force: true }); }
}
