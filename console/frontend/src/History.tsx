import { useEffect, useState } from 'react';
import { ApiError, api, wire, type HistoryDetail, type HistoryEntry, type HistoryPage } from './api';
import { historyHash, durationLabel, entryRecordingBytes, stateLabel, stateTone, timeLabel } from './model';
import { Result } from './Result';
import { RunActivity } from './RunActivity';
import { RunMetrics, bytesLabel } from './RunMetrics';
import { ActionButton } from './ui/ActionButton';
import { StatusLabel } from './ui/StatusLabel';

const { HistoryState: State } = wire;
const filters = [State.HISTORY_COMPLETED, State.HISTORY_FAILED, State.HISTORY_CANCELLED, State.HISTORY_BLOCKED, State.HISTORY_INTERRUPTED, State.HISTORY_RUNNING];
const failure = (error: unknown) => error instanceof ApiError && error.code === 'Missing' ? 'This run is not in the history.' : error instanceof ApiError && error.code === 'Unauthorized' ? 'Session ended. Open the access link printed by the running Console.' : 'History could not be loaded. Try again.';

export function Degraded({ reason }: { reason: string }) {
  return <p className="notice" role="status">History is degraded: {reason}. Runs still start, stop and recover normally. Completed runs return to this list once the database opens.</p>;
}

function Row({ entry }: { entry: HistoryEntry }) {
  return <tr>
    <td><StatusLabel label={stateLabel(entry.state)} tone={stateTone(entry.state)}/></td>
    <td><a className="mono break" href={historyHash(entry.runId)}>{entry.projectDirectory || entry.runId}</a>{entry.profiles.length > 0 && <div className="muted">{entry.profiles.join(', ')}</div>}{entry.errorSummary && <div className="history-error" title={entry.errorSummary}>{entry.errorSummary}</div>}</td>
    <td className="hide-narrow">{timeLabel(entry.startedAtMs)}</td>
    <td>{durationLabel(entry)}</td>
    <td className="hide-narrow">{bytesLabel(entryRecordingBytes(entry))}</td>
  </tr>;
}

/** Filtered run table; loading, failure, degraded and empty states come from its props. */
export function HistoryTable({ page, loading, error, state, project, setState, setProject, more }: {
  page: HistoryPage | null; loading: boolean; error: string; state: number; project: string;
  setState: (state: number) => void; setProject: (project: string) => void; more: () => void;
}) {
  const filtered = state !== 0 || project !== '';
  return <>
    <div className="heading"><div><h1>Experiment history</h1></div></div>
    {page?.degraded && <Degraded reason={page.degraded}/>}
    {!!page?.unreadable && <p className="notice" role="status">{page.unreadable} saved {page.unreadable === 1 ? 'run' : 'runs'} could not be read. The files stay on the host.</p>}
    {error && <p className="notice failure-notice" role="alert">{error}</p>}
    <section className="panel">
      <div className="history-filters">
        <div><label htmlFor="history-state">Status</label><select id="history-state" value={state} onChange={event => setState(Number(event.target.value))}><option value={0}>All</option>{filters.map(item => <option key={item} value={item}>{stateLabel(item)}</option>)}</select></div>
        <div><label htmlFor="history-project">Project</label><select id="history-project" value={project} onChange={event => setProject(event.target.value)}><option value="">All</option>{page?.projects.map(item => <option key={item} value={item}>{item}</option>)}</select></div>
      </div>
      {page && page.entries.length > 0 && <table className="history-table"><thead><tr><th className="history-status">Status</th><th>Project</th><th className="hide-narrow">Started</th><th>Duration</th><th className="hide-narrow">Recordings</th></tr></thead><tbody>{page.entries.map(entry => <Row key={entry.runId} entry={entry}/>)}</tbody></table>}
      {loading && <p className="muted" role="status">Loading history…</p>}
      {!loading && !error && page?.entries.length === 0 && !page.degraded && <p className="muted">{filtered ? 'No runs match these filters.' : 'No runs yet. Finished and interrupted experiments appear here.'}</p>}
      {page?.nextCursor && <div className="actions"><ActionButton variant="secondary" disabled={loading} onClick={more}>Load more</ActionButton></div>}
    </section>
  </>;
}

export function HistoryList({ refresh }: { refresh: string }) {
  const [state, setState] = useState(0);
  const [project, setProject] = useState('');
  const [page, setPage] = useState<HistoryPage | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const load = async (cursor: string, current: () => boolean) => {
    setLoading(true);
    try {
      const next = await api.history({ state, project, cursor });
      if (!current()) return;
      setError('');
      setPage(previous => cursor && previous ? wire.HistoryPage.create({ ...next, entries: [...previous.entries, ...next.entries] }) : next);
    } catch (failed) { if (current()) setError(failure(failed)); }
    finally { if (current()) setLoading(false); }
  };
  useEffect(() => { let live = true; void load('', () => live); return () => { live = false; }; }, [state, project, refresh]);
  return <HistoryTable page={page} loading={loading} error={error} state={state} project={project} setState={setState} setProject={setProject} more={() => void load(page!.nextCursor, () => true)}/>;
}

/** One stored run: the live Result, timeline and metrics components over its saved status. */
export function HistoryRun({ detail, runAgain, runActive }: { detail: HistoryDetail; runAgain: () => void; runActive: boolean }) {
  const entry = detail.entry!, status = detail.status!;
  return <>
    <p><a className="history-back" href={historyHash()}>Back to history</a></p>
    {entry.state === State.HISTORY_RUNNING && <p className="notice" role="status">This run is in progress. <a href="#/">View live run</a></p>}
    <Result snapshot={status} newRun={runAgain} past={{
      title: entry.state === State.HISTORY_BLOCKED ? 'Cleanup unverified' : `Run ${stateLabel(entry.state).toLowerCase()}`, action: 'Run again', disabled: runActive, artifacts: entry.artifacts,
      aside: <section className="panel"><div className="panel-heading"><h2>Run details</h2><StatusLabel label={stateLabel(entry.state)} tone={stateTone(entry.state)}/></div><dl className="detail-grid"><dt>Project</dt><dd>{entry.projectDirectory || 'Unknown'}</dd><dt>Profiles</dt><dd>{entry.profiles.join(', ') || 'Default'}</dd><dt>System apps</dt><dd>{entry.systemApps?.ids?.join(', ') || 'None'}</dd><dt>Started</dt><dd>{timeLabel(entry.startedAtMs)}</dd><dt>Finished</dt><dd>{entry.finishedAtMs == null ? 'Unknown' : timeLabel(entry.finishedAtMs)}</dd><dt>Duration</dt><dd>{durationLabel(entry)}</dd></dl>{entry.errorSummary && <p className="notice failure-notice">{entry.errorSummary}</p>}</section>,
    }}/>
    {/* A saved snapshot of a live run is stale and would read as interrupted; the banner links to the live view. */}
    {entry.state !== State.HISTORY_RUNNING && <div className="columns history-activity"><div><RunActivity snapshot={status} stopping={false} disconnected={false} historical/></div><aside><RunMetrics recordingEnabled={status.recordings.length > 0} samples={status.metrics} historical incomplete={status.metricsIncomplete} recordingOnly/></aside></div>}
  </>;
}

export function HistoryRunPage({ id, refresh, runAgain, runActive }: { id: string; refresh: string; runAgain: (entry: HistoryEntry) => void; runActive: boolean }) {
  const [detail, setDetail] = useState<HistoryDetail | null>(null);
  const [error, setError] = useState('');
  useEffect(() => {
    let live = true;
    api.historyRun(id).then(next => { if (live) { setDetail(next); setError(''); } }, failed => { if (live) setError(failure(failed)); });
    return () => { live = false; };
  }, [id, refresh]);
  if (error) return <><p><a className="history-back" href={historyHash()}>Back to history</a></p><p className="notice failure-notice" role="alert">{error}</p></>;
  if (detail?.entry?.runId !== id) return <p className="muted" role="status">Loading run…</p>;
  return <HistoryRun detail={detail} runActive={runActive} runAgain={() => runAgain(detail.entry!)}/>;
}
