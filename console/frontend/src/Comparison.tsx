import { useEffect, useState } from 'react';
import { ApiError, wire, type HistoryComparison, type HistoryEntry, type ReviewComparison } from './api';
import { useConsole } from './context';
import { durationLabel, factLabel, factValue, historyHash, stateLabel, stateTone, timeLabel } from './model';
import { ActionButton } from './ui/ActionButton';
import { StatusLabel } from './ui/StatusLabel';

type Row = HistoryComparison['rows'][number];
const { ComparisonGroup: Group, ComparisonResult: Result } = wire;
const GROUPS = [
  { group: Group.COMPARISON_OUTCOME, title: 'Result' },
  { group: Group.COMPARISON_SETUP, title: 'Experiment setup' },
  { group: Group.COMPARISON_CONFIGURATION, title: 'Machine configuration' },
  { group: Group.COMPARISON_CONDITION, title: 'Machine condition' },
];
const failure = (error: unknown) => error instanceof ApiError && error.code === 'Missing' ? 'One of these runs is not in the history.' : error instanceof ApiError && error.code === 'Unauthorized' ? 'Session ended. Open the access link printed by the running Console.' : 'The comparison could not be loaded. Try again.';
const value = (row: Row, side: number) => {
  const text = [...new Set(row.cells[side]?.values ?? [])].map(item => factValue(row.name, item)).join(', ');
  return row.hidden ? 'Hidden' : text || (row.result === Result.COMPARISON_UNAVAILABLE ? 'Unavailable' : 'Not set');
};
const resultLabel = (row: Row) => row.result === Result.COMPARISON_SAME ? 'Same' : row.result === Result.COMPARISON_CHANGED ? 'Changed' : 'Unavailable';
const factsOnly = (rows: Row[]) => rows.filter(row => row.name !== 'machine_capture_error');

export function MachineCaptureErrors({ rows, sides }: { rows: Row[]; sides: string[] }) {
  return <>{rows.filter(row => row.name === 'machine_capture_error').flatMap(row => sides.flatMap((side, index) =>
    [...new Set(row.cells[index]?.values ?? [])].filter(Boolean).map(error => <p className="notice failure-notice" role="alert" key={`${side}:${error}`}>Machine capture unavailable ({side}): {error}</p>),
  ))}</>;
}

function Rows({ rows, warn, sides, verdict = true }: { rows: Row[]; warn: boolean; sides: string[]; verdict?: boolean }) {
  return <div className="comparison-rows">{rows.map(row => <div className={`comparison-row${row.result === Result.COMPARISON_CHANGED ? (warn ? ' comparison-warning' : ' comparison-changed') : ''}`} key={row.name}>
    <code className="break">{factLabel(row.name)}</code>
    {sides.map((label, side) => <span className="comparison-value" key={label}><span className="comparison-side">{label}</span>{value(row, side)}</span>)}
    {verdict && <span className="comparison-result">{resultLabel(row)}</span>}
  </div>)}</div>;
}
/** Machine condition differences are highlighted without warning counts or verdicts. */
function ConditionSection({ title, rows, sides }: { title: string; rows: Row[]; sides: string[] }) {
  rows = factsOnly(rows);
  return <section className="comparison-group"><h3>{title}</h3>{rows.length ? <Rows rows={rows} warn={false} sides={sides} verdict={false}/> : <p className="muted">Nothing recorded.</p>}</section>;
}
function GroupSection({ group, title, rows, warn, showMatching, sides }: { group: number; title: string; rows: Row[]; warn: boolean; showMatching: boolean; sides: string[] }) {
  rows = factsOnly(rows);
  const changed = rows.filter(row => row.result !== Result.COMPARISON_SAME), same = rows.filter(row => row.result === Result.COMPARISON_SAME);
  const count = rows.filter(row => row.result === Result.COMPARISON_CHANGED).length;
  return <section className="comparison-group" data-group={group}>
    <div className="panel-heading"><h3>{title}</h3>{warn && count > 0 ? <StatusLabel label={`${count} can change results`} tone="attention"/> : <span className="muted">{count} changed</span>}</div>
    {changed.length > 0 && <Rows rows={changed} warn={warn} sides={sides}/>}
    {same.length > 0 && !showMatching && <p className="muted">{same.length} matching</p>}
    {same.length > 0 && showMatching && <Rows rows={same} warn={false} sides={sides}/>}
    {rows.length === 0 && <p className="muted">Nothing recorded.</p>}
  </section>;
}
function RunHeader({ label, entry }: { label: string; entry: HistoryEntry }) {
  const { basePath } = useConsole();
  return <div className="comparison-run"><strong>{label}</strong> <a className="mono" href={historyHash(entry.runId, basePath)}>{entry.runId.slice(0, 8)}</a> <StatusLabel label={stateLabel(entry.state)} tone={stateTone(entry.state)}/><div className="muted">{timeLabel(entry.startedAtMs)} · {durationLabel(entry)}</div></div>;
}
/** Differences between two runs, grouped by what they mean. */
function ComparisonView({ comparison, clear }: { comparison: HistoryComparison; clear: string }) {
  const [showMatching, setShowMatching] = useState(false);
  const sides = comparison.sides.map(side => side.label);
  const missing = comparison.sides.filter(side => side.runs.some(run => !run.captured)).map(side => side.label);
  return <section className="panel comparison" aria-label="Run comparison">
    <div className="panel-heading"><h2>Compare runs</h2><div className="comparison-actions"><ActionButton variant="secondary" onClick={() => setShowMatching(!showMatching)}>{showMatching ? 'Show differences only' : 'Show all'}</ActionButton><a className="history-back" href={clear}>Clear selection</a></div></div>
    <div className="comparison-runs">{comparison.sides.flatMap(side => side.runs.map(run => <RunHeader key={run.entry!.runId} label={side.label} entry={run.entry!}/>))}</div>
    {missing.length > 0 && <p className="notice">No machine capture for run {missing.join(' or ')}.</p>}
    <MachineCaptureErrors rows={comparison.rows} sides={sides}/>
    {GROUPS.map(({ group, title }) => group === Group.COMPARISON_CONDITION ? <ConditionSection key={group} title={title} rows={comparison.rows.filter(row => row.group === group)} sides={sides}/> : <GroupSection key={group} group={group} title={title} rows={comparison.rows.filter(row => row.group === group)} warn={group === Group.COMPARISON_CONFIGURATION} showMatching={showMatching} sides={sides}/>)}
    {comparison.truncated && <p className="muted">Some matching facts are not shown.</p>}
  </section>;
}
/** Values only, for facts with nothing to compare against. */
function CurrentFacts({ title, rows }: { title: string; rows: Row[] }) {
  rows = factsOnly(rows);
  return <section className="comparison-group"><h3>{title}</h3><dl className="detail-grid">{rows.map(row => <div className="comparison-fact" key={row.name}><dt className="break">{factLabel(row.name)}</dt><dd>{value(row, 1)}</dd></div>)}</dl></section>;
}
/** What this review would run on, compared with an earlier run; differences warn and never block. */
export function ReviewPreview({ comparison }: { comparison: ReviewComparison }) {
  const { basePath } = useConsole();
  const [showMatching, setShowMatching] = useState(false);
  const rows = (group: number) => comparison.rows.filter(row => row.group === group);
  const baseline = comparison.baseline;
  return <section className="panel comparison" aria-label="Machine review">
    <div className="panel-heading"><h2>Compared with</h2>{baseline && <ActionButton variant="secondary" onClick={() => setShowMatching(!showMatching)}>{showMatching ? 'Show differences only' : 'Show all'}</ActionButton>}</div>
    {baseline ? <p><a className="history-back mono" href={historyHash(baseline.runId, basePath)}>{baseline.runId.slice(0, 8)}</a> <span className="muted">{timeLabel(baseline.startedAtMs)}</span></p> : <p className="muted">No earlier capture to compare with.</p>}
    <MachineCaptureErrors rows={comparison.rows} sides={['Then', 'Now']}/>
    {baseline && <GroupSection group={Group.COMPARISON_SETUP} title="Experiment setup" rows={rows(Group.COMPARISON_SETUP)} warn={false} showMatching={showMatching} sides={['Then', 'Now']}/>}
    {baseline
      ? <GroupSection group={Group.COMPARISON_CONFIGURATION} title="Machine configuration" rows={rows(Group.COMPARISON_CONFIGURATION)} warn showMatching={showMatching} sides={['Then', 'Now']}/>
      : <CurrentFacts title="Machine configuration" rows={rows(Group.COMPARISON_CONFIGURATION)}/>}
    <CurrentFacts title="Machine condition" rows={rows(Group.COMPARISON_CONDITION)}/>
    {comparison.truncated && <p className="muted">Some matching facts are not shown.</p>}
  </section>;
}
/** What changed on the machine during the run that just ended; configuration changes warn. */
export function MachineChanges({ rows }: { rows: Row[] }) {
  const inGroup = (group: number) => rows.filter(row => row.group === group);
  const configuration = inGroup(Group.COMPARISON_CONFIGURATION);
  // A failed or oversized capture leaves the configuration unknown, not unchanged.
  const captureFailed = rows.some(row => row.name === 'machine_capture_error' && row.cells.some(cell => (cell.values ?? []).some(Boolean)));
  return <section className="panel comparison machine-changes" aria-label="Machine changes during this run">
    <h2>Machine during this run</h2>
    <MachineCaptureErrors rows={rows} sides={['Before', 'After']}/>
    {configuration.length
      ? <GroupSection group={Group.COMPARISON_CONFIGURATION} title="Machine configuration" rows={configuration} warn showMatching sides={['Before', 'After']}/>
      : <p className="muted">{captureFailed ? 'Machine configuration changes are unavailable.' : 'Machine configuration did not change.'}</p>}
    <GroupSection group={Group.COMPARISON_CONDITION} title="Machine condition" rows={inGroup(Group.COMPARISON_CONDITION)} warn={false} showMatching sides={['Before', 'After']}/>
  </section>;
}
export function ComparisonPanel({ a, b, refresh, clear }: { a: string; b: string; refresh: string; clear: string }) {
  const { client } = useConsole();
  const [comparison, setComparison] = useState<HistoryComparison | null>(null);
  const [error, setError] = useState('');
  useEffect(() => {
    let live = true;
    setComparison(null); setError('');
    client.historyCompare(a, b).then(next => { if (live) setComparison(next); }, failed => { if (live) setError(failure(failed)); });
    return () => { live = false; };
  }, [a, b, refresh]);
  if (error) return <section className="panel comparison"><div className="panel-heading"><h2>Compare runs</h2><a className="history-back" href={clear}>Clear selection</a></div><p className="notice failure-notice" role="alert">{error}</p></section>;
  if (!comparison) return <section className="panel comparison"><p className="muted" role="status">Comparing runs…</p></section>;
  return <ComparisonView comparison={comparison} clear={clear}/>;
}
