import { constants, closeSync, fchmodSync, fstatSync, openSync } from 'node:fs';
import { mkdir, open, realpath } from 'node:fs/promises';
import { join } from 'node:path';
import { capabilities, wire, type Database } from './native.js';

const MAX_MARKER_BYTES = 64 * 1024;
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
export function migrate(db: Database, steps = MIGRATIONS) {
  const version = () => Number(db.query('PRAGMA user_version')[0].user_version);
  for (let next = version(); next < steps.length; next++) {
    db.exec('BEGIN IMMEDIATE');
    try {
      db.exec(steps[next]);
      db.exec(`PRAGMA user_version = ${next + 1}`);
      db.exec('COMMIT');
    } catch (error) {
      db.exec('ROLLBACK');
      throw error;
    }
  }
}

/**
 * Console's durable state: `<data_root>/console.db`, with recordings beside it under `runs/`.
 * Errors: open and writes throw; Console must not start a run it cannot record.
 */
export class Store {
  private constructor(
    readonly root: string,
    readonly db: Database,
    private lock: { close(): void },
  ) {}
  static async open(path: string) {
    await mkdir(path, { recursive: true, mode: 0o700 });
    const root = await realpath(path);
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
      const db = capabilities.openDatabase(file);
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
    this.db.close();
    this.lock.close();
  }
  /** The latest run's marker, or null when absent or unreadable. */
  marker() {
    const row = this.db.query('SELECT value FROM marker WHERE id = 1')[0];
    if (!row) return null;
    try {
      return wire.Marker.decode(row.value as Buffer);
    } catch {
      return null;
    }
  }
  /** Replace the marker durably. */
  write(marker: typeof wire.Marker.prototype) {
    const bytes = wire.Marker.encode(marker).finish();
    if (bytes.length > MAX_MARKER_BYTES) throw new Error('marker too large');
    this.db.query('INSERT INTO marker (id, value) VALUES (1, ?) ON CONFLICT (id) DO UPDATE SET value = excluded.value', [Buffer.from(bytes)]);
  }
  /** One run's saved status, or null when absent or unreadable. */
  activity(id: string) {
    const row = this.db.query('SELECT detail FROM runs WHERE run_id = ?', [id])[0];
    if (!row) return null;
    try {
      return wire.Status.decode(row.detail as Buffer);
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
