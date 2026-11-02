import { constants, createReadStream, closeSync, fstatSync } from 'node:fs';
import { mkdir, open, rename, rm, realpath } from 'node:fs/promises';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { capabilities, wire, type Snapshot } from './native.js';

const MAX_MARKER_BYTES = 64 * 1024,
  MAX_ACTIVITY_BYTES = 2 * 1024 * 1024;
export async function writeOnce(path: string, bytes: Uint8Array) {
  const file = await open(
    path,
    constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY | constants.O_NOFOLLOW,
    0o600,
  );
  try {
    await file.writeFile(bytes);
    await file.sync();
  } finally {
    await file.close();
  }
}
async function bounded(root: string, path: string, limit: number) {
  const fd = capabilities.openFile(root, path),
    source = createReadStream('', { fd, autoClose: true, start: 0, end: limit });
  const chunks: Buffer[] = [];
  let size = 0;
  for await (const bytes of source) {
    size += bytes.length;
    if (size > limit) throw new Error('checkpoint too large');
    chunks.push(bytes);
  }
  return Buffer.concat(chunks);
}
/** Persist browser metadata independently of repository admission and recovery. */
export class Store {
  private constructor(
    readonly root: string,
    private lock: { close(): void },
  ) {}
  static async open(path: string) {
    await mkdir(path, { recursive: true, mode: 0o700 });
    const root = await realpath(path);
    return new Store(root, capabilities.lockFile(join(root, 'console.lock')));
  }
  close() {
    this.lock.close();
  }
  async marker() {
    try {
      return wire.Marker.decode(await bounded(this.root, 'current.pb', MAX_MARKER_BYTES));
    } catch {
      return null;
    }
  }
  async activity(id: string) {
    if (!/^[a-f0-9]{32}$/.test(id)) return null;
    try {
      return wire.Status.decode(
        await bounded(join(this.root, 'runs'), `${id}/activity.pb`, MAX_ACTIVITY_BYTES),
      );
    } catch {
      return null;
    }
  }
  async write(marker: typeof wire.Marker.prototype) {
    const bytes = wire.Marker.encode(marker).finish();
    if (bytes.length > MAX_MARKER_BYTES) throw new Error('marker too large');
    await this.atomic('current.pb', bytes);
  }
  async checkpoint(status: Snapshot) {
    if (!/^[a-f0-9]{32}$/.test(status.runId)) throw new Error('invalid checkpoint run id');
    const bytes = wire.Status.encode(status).finish();
    if (bytes.length > MAX_ACTIVITY_BYTES) throw new Error('activity too large');
    await this.atomic(join('runs', status.runId, 'activity.pb'), bytes);
  }
  private async atomic(relative: string, bytes: Uint8Array) {
    const target = join(this.root, relative),
      temporary = target + '.' + randomUUID() + '.tmp';
    try {
      await writeOnce(temporary, bytes);
      await rename(temporary, target);
      const parent = await open(join(target, '..'), constants.O_RDONLY | constants.O_DIRECTORY);
      try {
        await parent.sync();
      } finally {
        await parent.close();
      }
    } finally {
      await rm(temporary, { force: true });
    }
  }
  available(id: string, path: string) {
    return this.size(id, path) !== null;
  }
  /** Size of one regular file beneath a run, or null when it cannot be opened safely. */
  size(id: string, path: string) {
    return this.stat(join(this.root, 'runs'), `${id}/${path}`)?.size ?? null;
  }
  /** Modification time of a run's checkpoint in milliseconds, or null without one. */
  activityTime(id: string) {
    const time = this.stat(join(this.root, 'runs'), `${id}/activity.pb`)?.mtimeMs;
    return time === undefined ? null : Math.floor(time);
  }
  markerTime() {
    const time = this.stat(this.root, 'current.pb')?.mtimeMs;
    return time === undefined ? null : Math.floor(time);
  }
  private stat(root: string, path: string) {
    try {
      const fd = capabilities.openFile(root, path);
      try {
        return fstatSync(fd);
      } finally {
        closeSync(fd);
      }
    } catch {
      return null;
    }
  }
}
