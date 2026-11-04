import { useEffect, useState } from 'react';
import { ApiError, api, wire, type HistoryComparison, type HistoryEntry, type ReviewComparison } from './api';
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

function Rows({ rows, warn, sides, verdict = true }: { rows: Row[]; warn: boolean; sides: string[]; verdict?: boolean }) {
  return <div className="comparison-rows">{rows.map(row => <div className={`comparison-row${warn && row.result === Result.COMPARISON_CHANGED ? ' comparison-warning' : ''}`} key={row.name}>
    <code className="break">{factLabel(row.name)}</code>
    {sides.map((label, side) => <span className="comparison-value" key={label}><span className="comparison-side">{label}</span>{value(row, side)}</span>)}
    {verdict && <span className="comparison-result">{resultLabel(row)}</span>}
  </div>)}</div>;
}
/** Machine condition is expected to vary, so it reads as plain values with no count or verdict. */
function ConditionSection({ title, rows, sides }: { title: string; rows: Row[]; sides: string[] }) {
  return <section className="comparison-group"><h3>{title}</h3><Rows rows={rows} warn={false} sides={sides} verdict={false}/></section>;
}
function GroupSection({ group, title, rows, warn, showMatching, sides }: { group: number; title: string; rows: Row[]; warn: boolean; showMatching: boolean; sides: string[] }) {
  const changed = rows.filter(row => row.result !== Result.COMPARISON_SAME), same = rows.filter(row => row.result === Result.COMPARISON_SAME);
  const count = rows.filter(row => row.result === Result.COMPARISON_CHANGED).length;
  return <section className="comparison-group" data-group={group}>
    <div className="panel-heading"><h3>{title}</h3>{warn && count > 0 ? <StatusLabel label={`${count} can change results`} tone="attention"/> : <span className="muted">{count} changed</span>}</div>
    {changed.length > 0 && <Rows rows={changed} warn={warn} sides={sides}/>}
    {same.length > 0 && (showMatching ? <Rows rows={same} warn={false} sides={sides}/> : <p className="comparison-matching muted">{same.length} matching</p>)}
    {rows.length === 0 && <p className="muted">Nothing recorded.</p>}
  </section>;
}
function RunHeader({ label, entry }: { label: string; entry: HistoryEntry }) {
  return <div className="comparison-run"><strong>{label}</strong> <a className="mono" href={historyHash(entry.runId)}>{entry.runId.slice(0, 8)}</a> <StatusLabel label={stateLabel(entry.state)} tone={stateTone(entry.state)}/><div className="muted">{timeLabel(entry.startedAtMs)} · {durationLabel(entry)}</div></div>;
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
    {GROUPS.map(({ group, title }) => group === Group.COMPARISON_CONDITION ? <ConditionSection key={group} title={title} rows={comparison.rows.filter(row => row.group === group)} sides={sides}/> : <GroupSection key={group} group={group} title={title} rows={comparison.rows.filter(row => row.group === group)} warn={group === Group.COMPARISON_CONFIGURATION} showMatching={showMatching} sides={sides}/>)}
    {comparison.truncated && <p className="muted">Some matching facts are not shown.</p>}
  </section>;
}
/** Values only, for facts with nothing to compare against. */
function CurrentFacts({ title, rows }: { title: string; rows: Row[] }) {
  return <section className="comparison-group"><h3>{title}</h3><dl className="detail-grid">{rows.map(row => <div className="comparison-fact" key={row.name}><dt className="break">{factLabel(row.name)}</dt><dd>{value(row, 1)}</dd></div>)}</dl></section>;
}
/** What this review would run on, compared with an earlier run; differences warn and never block. */
export function ReviewPreview({ comparison }: { comparison: ReviewComparison }) {
  const [showMatching, setShowMatching] = useState(false);
  const rows = (group: number) => comparison.rows.filter(row => row.group === group);
  const baseline = comparison.baseline;
  return <section className="panel comparison" aria-label="Machine review">
    <div className="panel-heading"><h2>Compared with</h2>{baseline && <ActionButton variant="secondary" onClick={() => setShowMatching(!showMatching)}>{showMatching ? 'Show differences only' : 'Show all'}</ActionButton>}</div>
    {baseline ? <p><a className="history-back mono" href={historyHash(baseline.runId)}>{baseline.runId.slice(0, 8)}</a> <span className="muted">{timeLabel(baseline.startedAtMs)}</span></p> : <p className="muted">No earlier capture to compare with.</p>}
    {baseline && <GroupSection group={Group.COMPARISON_SETUP} title="Experiment setup" rows={rows(Group.COMPARISON_SETUP)} warn={false} showMatching={showMatching} sides={['Then', 'Now']}/>}
    {baseline
      ? <GroupSection group={Group.COMPARISON_CONFIGURATION} title="Machine configuration" rows={rows(Group.COMPARISON_CONFIGURATION)} warn showMatching={showMatching} sides={['Then', 'Now']}/>
      : <CurrentFacts title="Machine configuration" rows={rows(Group.COMPARISON_CONFIGURATION)}/>}
    <CurrentFacts title="Machine condition" rows={rows(Group.COMPARISON_CONDITION)}/>
    {comparison.truncated && <p className="muted">Some matching facts are not shown.</p>}
  </section>;
}
export function ComparisonPanel({ a, b, refresh, clear }: { a: string; b: string; refresh: string; clear: string }) {
  const [comparison, setComparison] = useState<HistoryComparison | null>(null);
  const [error, setError] = useState('');
  useEffect(() => {
    let live = true;
    setComparison(null); setError('');
    api.historyCompare(a, b).then(next => { if (live) setComparison(next); }, failed => { if (live) setError(failure(failed)); });
    return () => { live = false; };
  }, [a, b, refresh]);
  if (error) return <section className="panel comparison"><div className="panel-heading"><h2>Compare runs</h2><a className="history-back" href={clear}>Clear selection</a></div><p className="notice failure-notice" role="alert">{error}</p></section>;
  if (!comparison) return <section className="panel comparison"><p className="muted" role="status">Comparing runs…</p></section>;
  return <ComparisonView comparison={comparison} clear={clear}/>;
}
