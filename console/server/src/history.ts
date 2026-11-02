import { closeSync, constants, fchmodSync, openSync } from 'node:fs';
import { readdir } from 'node:fs/promises';
import { join } from 'node:path';
import type { DatabaseSync } from 'node:sqlite';
import { LIMITS } from './generated/transport.js';
import { wire, type Snapshot } from './native.js';
import type { Store } from './store.js';

const { HistoryState: State, Phase } = wire;
type Row = { entry: Uint8Array; detail: Uint8Array };
type Marker = typeof wire.Marker.prototype;

/** Newest runs kept in the index. Pruning removes rows only; recordings stay on disk. */
export const MAX_HISTORY_RUNS = 1000;
const MAX_DETAIL_BYTES = 2 * 1024 * 1024,
  MAX_ERROR_LENGTH = 1024,
  MAX_PAGE_BYTES = 1024 * 1024,
  MAX_PROJECTS = 100,
  RUN_ID = /^[a-f0-9]{32}$/;

/** Each step moves `user_version` up by one and runs in one transaction. */
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
   CREATE INDEX runs_by_state ON runs (state, started_at_ms DESC, run_id DESC);`,
];

/** Apply pending steps. Throws on a newer schema or a failed step, leaving the file at its last complete version. */
export function migrate(db: DatabaseSync, steps = MIGRATIONS) {
  const version = () => Number((db.prepare('PRAGMA user_version').get() as { user_version: number }).user_version);
  if (version() > steps.length) throw new Error(`console.db schema ${version()} is newer than this Console`);
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

const reason = (error: unknown) => String((error as Error)?.message ?? error).slice(0, MAX_ERROR_LENGTH);

/** Map one Console status to its history result. */
export function historyState(status: Pick<Snapshot, 'phase' | 'outcome'>) {
  if (status.phase === Phase.BLOCKED) return State.HISTORY_BLOCKED;
  if (!status.outcome) return State.HISTORY_RUNNING;
  if (status.outcome.execution === 'Failed') return State.HISTORY_FAILED;
  return status.outcome.execution === 'Cancelled' ? State.HISTORY_CANCELLED : State.HISTORY_COMPLETED;
}
const failedStep = (status: Snapshot) => [...status.steps].reverse().find((step) => step.error)?.error ?? '';
const encodeDetail = (status: Snapshot) => {
  const bytes = wire.Status.encode(status).finish();
  if (bytes.length <= MAX_DETAIL_BYTES) return bytes;
  return wire.Status.encode({ ...status, steps: [], metrics: [], progressIncomplete: true, metricsIncomplete: true }).finish();
};
const started = (detail: { steps: Snapshot['steps']; metrics: Snapshot['metrics'] }, finishedAt: number) => {
  const elapsed = [...detail.steps, ...detail.metrics].reduce((most, row) => Math.max(most, Number(row.elapsedNs ?? 0)), 0);
  return Math.max(0, finishedAt - Math.round(elapsed / 1e6));
};

/**
 * Optional SQLite index of runs at `<data_root>/console.db`.
 * - Errors: no method throws for a database failure; `degraded` carries the reason.
 * - Authority: `current.pb` and `runs/<id>/activity.pb` stay authoritative; this file can be rebuilt from them.
 */
export class History {
  /** Last database failure; empty when healthy. */
  degraded = '';
  /** Checkpoints skipped by the last backfill because they could not be decoded. */
  unreadable = 0;
  private constructor(
    private store: Store,
    private db?: DatabaseSync,
  ) {}
  /** A history that records nothing and reports `why` as its degraded state. */
  static unavailable(store: Store, why: string) {
    const history = new History(store);
    history.degraded = why;
    return history;
  }
  /** Open, migrate and reconcile with the checkpoints on disk. */
  static async open(store: Store, marker: Marker | null) {
    let history: History | undefined;
    try {
      // Loaded here so a Node.js without node:sqlite only loses history.
      const { DatabaseSync } = await import('node:sqlite');
      const path = join(store.root, 'console.db');
      // SQLite gives the -wal and -shm files this file's mode.
      const fd = openSync(path, constants.O_CREAT | constants.O_RDWR | constants.O_NOFOLLOW, 0o600);
      try {
        fchmodSync(fd, 0o600);
      } finally {
        closeSync(fd);
      }
      const db = new DatabaseSync(path);
      history = new History(store, db);
      db.exec('PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 250');
      migrate(db);
    } catch (error) {
      history?.close();
      return History.unavailable(store, reason(error));
    }
    const opened = history;
    await opened.attempt(() => opened.reconcile(marker));
    return opened;
  }
  close() {
    try {
      this.db?.close();
    } catch {}
    this.db = undefined;
  }
  /** Save one run transition. `startedAt` is ignored once the row exists. */
  record(status: Snapshot, startedAt: number, finishedAt?: number, error = '') {
    this.attemptSync(() => {
      const state = finishedAt === undefined ? State.HISTORY_RUNNING : historyState(status);
      this.upsert(status, state, this.startedAt(status.runId) ?? startedAt, finishedAt, error || failedStep(status));
    });
  }
  /** Return one page, newest first; a failed index yields `degraded` and no rows. */
  list(query: typeof wire.HistoryQuery.prototype) {
    const page = wire.HistoryPage.create({ unreadable: this.unreadable });
    this.read(() => {
      const limit = query.limit || LIMITS.historyPageDefault;
      const [time, id] = query.cursor ? query.cursor.split(':') : [Number.MAX_SAFE_INTEGER, ''];
      const rows = this.db!.prepare(
        `SELECT run_id, started_at_ms, entry FROM runs
         WHERE (started_at_ms, run_id) < (?, ?) AND (? = 0 OR state = ?) AND (? = '' OR project_directory = ?)
         ORDER BY started_at_ms DESC, run_id DESC LIMIT ?`,
      ).all(Number(time), id || 'g', query.state, query.state, query.project, query.project, limit + 1) as {
        run_id: string;
        started_at_ms: number;
        entry: Uint8Array;
      }[];
      let bytes = 0;
      for (const row of rows) {
        bytes += row.entry.length;
        if (page.entries.length === limit || bytes > MAX_PAGE_BYTES) {
          const last = rows[page.entries.length - 1];
          page.nextCursor = `${last.started_at_ms}:${last.run_id}`;
          break;
        }
        page.entries.push(wire.HistoryEntry.decode(row.entry));
      }
      page.projects = (
        this.db!.prepare('SELECT DISTINCT project_directory AS p FROM runs WHERE p != \'\' ORDER BY p LIMIT ?').all(MAX_PROJECTS) as { p: string }[]
      ).map((row) => row.p);
    });
    page.degraded = this.degraded;
    return page;
  }
  /** Return one run with artifact sizes re-read from disk, or null when unknown. */
  detail(id: string) {
    let found: typeof wire.HistoryDetail.prototype | null = null;
    this.read(() => {
      const row = this.db!.prepare('SELECT entry, detail FROM runs WHERE run_id = ?').get(id) as Row | undefined;
      if (!row) return;
      const entry = wire.HistoryEntry.decode(row.entry),
        status = wire.Status.decode(row.detail);
      entry.artifacts = this.artifacts(status);
      status.reportAvailable = this.store.available(id, 'console-report.json');
      found = wire.HistoryDetail.create({ entry, status });
    });
    return found as typeof wire.HistoryDetail.prototype | null;
  }
  private upsert(status: Snapshot, state: number, startedAt: number, finishedAt: number | undefined, error: string) {
    const entry = wire.HistoryEntry.create({
      runId: status.runId,
      projectDirectory: status.projectDirectory,
      profiles: status.profiles,
      systemApps: status.systemApps,
      startedAtMs: startedAt,
      ...(finishedAt === undefined ? {} : { finishedAtMs: finishedAt }),
      state,
      errorSummary: error.slice(0, MAX_ERROR_LENGTH),
      artifacts: this.artifacts(status),
    });
    this.db!.prepare(
      `INSERT INTO runs VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT (run_id) DO UPDATE SET
         project_directory = excluded.project_directory, state = excluded.state,
         entry = excluded.entry, detail = excluded.detail`,
    ).run(status.runId, status.projectDirectory, state, startedAt, wire.HistoryEntry.encode(entry).finish(), encodeDetail(status));
    this.prune();
  }
  /** Bound the index; a run still open is never pruned. */
  private prune() {
    this.db!.prepare(
      `DELETE FROM runs WHERE state != ? AND run_id NOT IN (SELECT run_id FROM runs ORDER BY started_at_ms DESC, run_id DESC LIMIT ?)`,
    ).run(State.HISTORY_RUNNING, MAX_HISTORY_RUNS);
  }
  private startedAt(id: string) {
    return (this.db!.prepare('SELECT started_at_ms AS t FROM runs WHERE run_id = ?').get(id) as { t: number } | undefined)?.t;
  }
  private artifacts(status: Snapshot) {
    const done = status.outcome != null;
    const finalized = status.outcome?.recordingFinalized === true;
    const declared = status.recordings.map((output) => {
      const size = this.store.size(status.runId, output.path!);
      return wire.HistoryArtifact.create({ id: output.id!, ...(size === null ? {} : { sizeBytes: size }), downloadable: finalized && size !== null });
    });
    const report = this.store.size(status.runId, 'console-report.json');
    return report === null ? declared : [...declared, wire.HistoryArtifact.create({ id: 'report', sizeBytes: report, downloadable: done })];
  }
  /** Rebuild from disk: checkpoints close open rows and add unknown runs; open rows left over were interrupted. */
  private async reconcile(marker: Marker | null) {
    const db = this.db!;
    const settled = new Set(
      (db.prepare('SELECT run_id FROM runs WHERE state != ?').all(State.HISTORY_RUNNING) as { run_id: string }[]).map((row) => row.run_id),
    );
    const names = await readdir(join(this.store.root, 'runs')).catch(() => [] as string[]);
    for (const id of names) {
      if (!RUN_ID.test(id) || settled.has(id)) continue;
      const finishedAt = this.store.activityTime(id);
      if (finishedAt === null) continue;
      // A full index would prune this run again at once; skip it so each open stays cheap.
      if (this.isFull() && this.startedAt(id) === undefined && finishedAt < this.oldest()) continue;
      const saved = await this.store.activity(id);
      if (!saved || saved.runId !== id || !saved.outcome) {
        this.unreadable++;
        continue;
      }
      this.upsert(saved, historyState(saved), this.startedAt(id) ?? started(saved, finishedAt), finishedAt, failedStep(saved));
      settled.add(id);
    }
    if (marker && RUN_ID.test(marker.runId) && !settled.has(marker.runId)) this.fromMarker(marker);
    const open = db.prepare('SELECT detail, started_at_ms AS t FROM runs WHERE state = ?').all(State.HISTORY_RUNNING) as { detail: Uint8Array; t: number }[];
    for (const row of open) {
      const status = wire.Status.decode(row.detail);
      this.upsert(status, State.HISTORY_INTERRUPTED, row.t, undefined, 'Console stopped before this run reported a result.');
    }
  }
  /** Index the last run named by `current.pb` when it has no checkpoint. */
  private fromMarker(marker: Marker) {
    const time = this.store.markerTime() ?? Date.now();
    const status = wire.Status.create({
      phase: marker.phase,
      runId: marker.runId,
      outcome: marker.outcome,
      projectDirectory: marker.projectDirectory,
      profiles: marker.profiles,
      systemApps: marker.systemApps,
      recordings: marker.recordings,
    });
    if (marker.outcome) this.upsert(status, historyState(status), this.startedAt(marker.runId) ?? time, time, '');
    else if (this.startedAt(marker.runId) === undefined) this.upsert(status, State.HISTORY_RUNNING, time, undefined, '');
  }
  private isFull() {
    return (this.db!.prepare('SELECT count(*) AS n FROM runs').get() as { n: number }).n >= MAX_HISTORY_RUNS;
  }
  private oldest() {
    return (this.db!.prepare('SELECT min(started_at_ms) AS t FROM runs').get() as { t: number }).t;
  }
  /** A successful read leaves `degraded` alone: it does not prove writes work. */
  private read(work: () => void) {
    if (!this.db) return;
    try {
      work();
    } catch (error) {
      this.degraded = reason(error);
    }
  }
  private attemptSync(work: () => void) {
    if (!this.db) return;
    try {
      work();
      this.degraded = '';
    } catch (error) {
      this.degraded = reason(error);
    }
  }
  private async attempt(work: () => Promise<void>) {
    try {
      await work();
      this.degraded = '';
    } catch (error) {
      this.degraded = reason(error);
    }
  }
}
