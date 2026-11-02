import { constants, closeSync, fchmodSync, fstatSync, openSync } from 'node:fs';
import { mkdir, realpath, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { capabilities, wire } from './native.js';

const MAX_MARKER_BYTES = 64 * 1024;
/**
 * Schema steps; each moves `user_version` up by one and runs in one transaction.
 * Additive only: rows are protobuf blobs plus query columns, so older Consoles keep reading.
 */
export const MIGRATIONS = [
  `CREATE TABLE runs (
     run_id TEXT PRIMARY KEY,
     project_directory TEXT NOT NULL,
     state INTEGER NOT NULL,
     started_at_ms INTEGER NOT NULL,
     entry BLOB NOT NULL,
     detail BLOB NOT NULL
   ) STRICT;
   CREATE INDEX runs_by_time ON runs (started_at_ms DESC, run_id DESC);
   CREATE INDEX runs_by_project ON runs (project_directory, started_at_ms DESC, run_id DESC);
   CREATE INDEX runs_by_state ON runs (state, started_at_ms DESC, run_id DESC);
   CREATE TABLE marker (id INTEGER PRIMARY KEY CHECK (id = 1), value BLOB NOT NULL) STRICT;`,
];

/** Apply pending steps. A failed step throws and leaves the file at its last complete version. */
export function migrate(db: DatabaseSync, steps = MIGRATIONS) {
  const version = () => (db.prepare('PRAGMA user_version').get() as { user_version: number }).user_version;
  for (let next = version(); next < steps.length; next++)
    transaction(db, () => {
      db.exec(steps[next]);
      db.exec(`PRAGMA user_version = ${next + 1}`);
    });
}

/** Run `work` so its writes all commit or none do. Errors: rethrows after rolling back. */
function transaction(db: DatabaseSync, work: () => void) {
  db.exec('BEGIN IMMEDIATE');
  try {
    work();
    db.exec('COMMIT');
  } catch (error) {
    db.exec('ROLLBACK');
    throw error;
  }
}

/**
 * Console's durable state: `<data_root>/console.db`, with recordings beside it under `runs/`.
 * Errors: open rejects a data root others can access; open and writes throw, so no unrecorded run starts.
 */
export class Store {
  private closed = false;
  private constructor(
    readonly root: string,
    readonly db: DatabaseSync,
    private lock: { close(): void },
  ) {}
  static async open(path: string) {
    await mkdir(path, { recursive: true, mode: 0o700 });
    const root = await realpath(path);
    // SQLite opens console.db by path, so nobody else may be able to swap it.
    const info = await stat(root);
    if (info.uid !== process.geteuid!() || (info.mode & 0o077) !== 0)
      throw new Error(`Data root ${root} must be owned by this user and mode 0700`);
    const lock = capabilities.lockFile(join(root, 'console.lock'));
    try {
      const file = join(root, 'console.db');
      // SQLite gives the -wal and -shm files this file's mode.
      const fd = openSync(file, constants.O_CREAT | constants.O_RDWR | constants.O_NOFOLLOW, 0o600);
      try {
        fchmodSync(fd, 0o600);
      } finally {
        closeSync(fd);
      }
      const db = new DatabaseSync(file);
      try {
        // FULL keeps a marker written before execution across power loss.
        db.exec('PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA busy_timeout = 250');
        migrate(db);
      } catch (error) {
        db.close();
        throw error;
      }
      return new Store(root, db, lock);
    } catch (error) {
      lock.close();
      throw error;
    }
  }
  close() {
    if (!this.closed) this.db.close();
    this.closed = true;
    this.lock.close();
  }
  /** The latest run's marker, or null when absent or unreadable. */
  marker() {
    const row = this.db.prepare('SELECT value FROM marker WHERE id = 1').get() as { value: Uint8Array } | undefined;
    if (!row) return null;
    try {
      return wire.Marker.decode(row.value);
    } catch {
      return null;
    }
  }
  /** Replace the marker durably. */
  write(marker: typeof wire.Marker.prototype) {
    const bytes = wire.Marker.encode(marker).finish();
    if (bytes.length > MAX_MARKER_BYTES) throw new Error('marker too large');
    this.db.prepare('INSERT INTO marker (id, value) VALUES (1, ?) ON CONFLICT (id) DO UPDATE SET value = excluded.value').run(bytes);
  }
  /** Commit all of `work`'s writes or none of them. Errors: rethrows after rolling back. */
  transaction(work: () => void) {
    transaction(this.db, work);
  }
  /** One run's saved status, or null when absent or unreadable. */
  activity(id: string) {
    const row = this.db.prepare('SELECT detail FROM runs WHERE run_id = ?').get(id) as { detail: Uint8Array } | undefined;
    if (!row) return null;
    try {
      return wire.Status.decode(row.detail);
    } catch {
      return null;
    }
  }
  available(id: string, path: string) {
    return this.size(id, path) !== null;
  }
  /** Size of one regular file beneath a run, or null when it cannot be opened safely. */
  size(id: string, path: string) {
    try {
      const fd = capabilities.openFile(join(this.root, 'runs'), `${id}/${path}`);
      try {
        return fstatSync(fd).size;
      } finally {
        closeSync(fd);
      }
    } catch {
      return null;
    }
  }
}
