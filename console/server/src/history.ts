import { LIMITS } from './generated/transport.js';
import { browserError, execution, wire, type Snapshot } from './native.js';
import type { Store } from './store.js';

const { HistoryState: State, Phase } = wire;
type Row = { entry: Uint8Array; detail: Uint8Array };

const MAX_DETAIL_BYTES = 2 * 1024 * 1024,
  MAX_PAGE_BYTES = 1024 * 1024,
  MAX_CAPTURE_BYTES = 256 * 1024;
type Capture = typeof execution.RunCapture.prototype;

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
       WHERE (started_at_ms, run_id) < (?, ?) AND (? = 0 OR state = ?)
       ORDER BY started_at_ms DESC, run_id DESC LIMIT ?`,
    ).all(Number(time), id || 'g', query.state, query.state, limit + 1) as { run_id: string; started_at_ms: number; entry: Uint8Array }[];
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
  /** One run's capture; empty for an earlier or unreadable one. */
  capture(id: string): Capture {
    const row = this.store.db.prepare('SELECT capture FROM runs WHERE run_id = ?').get(id) as { capture: Uint8Array | null } | undefined;
    try {
      return row?.capture ? execution.RunCapture.decode(row.capture) : execution.RunCapture.create();
    } catch {
      return execution.RunCapture.create();
    }
  }
  /** Merge newly known parts into one run's capture. Errors: a database failure throws; an oversized capture is left unsaved. */
  saveCapture(id: string, parts: Partial<Capture>) {
    const bytes = execution.RunCapture.encode({ ...this.capture(id), ...parts }).finish();
    if (bytes.length > MAX_CAPTURE_BYTES) throw new Error('run capture too large');
    this.store.db.prepare('UPDATE runs SET capture = ? WHERE run_id = ?').run(bytes, id);
  }
  private upsert(status: Snapshot, state: number, startedAt: number, finishedAt: number | undefined, error: string) {
    const entry = wire.HistoryEntry.create({
      runId: status.runId,
      profiles: status.profiles,
      systemApps: status.systemApps,
      startedAtMs: startedAt,
      ...(finishedAt === undefined ? {} : { finishedAtMs: finishedAt }),
      state,
      errorSummary: browserError(error),
      artifacts: this.artifacts(status),
    });
    this.store.db.prepare(
      `INSERT INTO runs (run_id, state, started_at_ms, entry, detail) VALUES (?, ?, ?, ?, ?)
       ON CONFLICT (run_id) DO UPDATE SET
         state = excluded.state, entry = excluded.entry, detail = excluded.detail`,
    ).run(status.runId, state, startedAt, wire.HistoryEntry.encode(entry).finish(), encodeDetail(status));
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
