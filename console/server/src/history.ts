import { LIMITS } from './generated/transport.js';
import { wire, type Snapshot } from './native.js';
import type { Store } from './store.js';

const { HistoryState: State, Phase } = wire;
type Row = { entry: Uint8Array; detail: Uint8Array };

const MAX_DETAIL_BYTES = 2 * 1024 * 1024,
  MAX_ERROR_LENGTH = 1024,
  MAX_PAGE_BYTES = 1024 * 1024,
  MAX_PROJECTS = 100;

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
/**
 * Past and current runs, stored in `console.db` as protobuf blobs with indexed query columns.
 * Errors: database failures throw to the caller.
 */
export class History {
  constructor(private store: Store) {}
  /** A row still open when Console starts belongs to a run that never reported a result. */
  closeInterrupted() {
    const open = this.store.db.prepare('SELECT detail, started_at_ms AS t FROM runs WHERE state = ?').all(State.HISTORY_RUNNING) as { detail: Uint8Array; t: number }[];
    for (const row of open)
      this.upsert(wire.Status.decode(row.detail), State.HISTORY_INTERRUPTED, row.t, undefined, 'Console stopped before this run reported a result.');
  }
  /** Save one run transition. `startedAt` is ignored once the row exists. */
  record(status: Snapshot, startedAt: number, finishedAt?: number, error = '') {
    const state = finishedAt === undefined ? State.HISTORY_RUNNING : historyState(status);
    this.upsert(status, state, this.startedAt(status.runId) ?? startedAt, finishedAt, error || failedStep(status));
  }
  /** Return one page, newest first. */
  list(query: typeof wire.HistoryQuery.prototype) {
    const page = wire.HistoryPage.create();
    const limit = query.limit || LIMITS.historyPageDefault;
    const [time, id] = query.cursor ? query.cursor.split(':') : [Number.MAX_SAFE_INTEGER, ''];
    const rows = this.store.db.prepare(
      `SELECT run_id, started_at_ms, entry FROM runs
       WHERE (started_at_ms, run_id) < (?, ?) AND (? = 0 OR state = ?) AND (? = '' OR project_directory = ?)
       ORDER BY started_at_ms DESC, run_id DESC LIMIT ?`,
    ).all(Number(time), id || 'g', query.state, query.state, query.project, query.project, limit + 1) as { run_id: string; started_at_ms: number; entry: Uint8Array }[];
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
      this.store.db.prepare("SELECT DISTINCT project_directory AS p FROM runs WHERE p != '' ORDER BY p LIMIT ?").all(MAX_PROJECTS) as { p: string }[]
    ).map((row) => row.p);
    return page;
  }
  /** Return one run with artifact sizes re-read from disk, or null when unknown. */
  detail(id: string) {
    const row = this.store.db.prepare('SELECT entry, detail FROM runs WHERE run_id = ?').get(id) as Row | undefined;
    if (!row) return null;
    const entry = wire.HistoryEntry.decode(row.entry),
      status = wire.Status.decode(row.detail);
    entry.artifacts = this.artifacts(status);
    status.reportAvailable = this.store.available(id, 'console-report.json');
    return wire.HistoryDetail.create({ entry, status });
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
    this.store.db.prepare(
      `INSERT INTO runs (run_id, project_directory, state, started_at_ms, entry, detail) VALUES (?, ?, ?, ?, ?, ?)
       ON CONFLICT (run_id) DO UPDATE SET
         project_directory = excluded.project_directory, state = excluded.state,
         entry = excluded.entry, detail = excluded.detail`,
    ).run(status.runId, status.projectDirectory, state, startedAt, wire.HistoryEntry.encode(entry).finish(), encodeDetail(status));
  }
  private startedAt(id: string) {
    return (this.store.db.prepare('SELECT started_at_ms AS t FROM runs WHERE run_id = ?').get(id) as { t: number } | undefined)?.t;
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
}
