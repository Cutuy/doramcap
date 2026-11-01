import type { RunMetric } from './api';
import { Health, type CpuView, type FaultView, type LiveView } from './streams';

export const bytesLabel = (value: string | number | { toString(): string } | null | undefined) => {
  if (value == null) return 'Unavailable';
  const bytes = Number(value);
  if (!Number.isFinite(bytes) || bytes < 0) return 'Unavailable';
  const unit = bytes >= 1024 ** 3 ? 3 : bytes >= 1024 ** 2 ? 2 : bytes >= 1024 ? 1 : 0;
  return `${(bytes / 1024 ** unit).toFixed(unit ? 1 : 0)} ${['B', 'KiB', 'MiB', 'GiB'][unit]}`;
};
type Point = { seconds: number; value: number | null | undefined };
function Sparkline({ points, label, scale }: { points: Point[]; label: string; scale: (max: number) => string }) {
  const max = Math.max(1, ...points.map(point => point.value).filter((v): v is number => v != null && Number.isFinite(v)));
  const start = points[0]?.seconds ?? 0;
  const span = Math.max(1, (points.at(-1)?.seconds ?? 0) - start);
  let connected = false;
  const path = points.map(({ seconds, value }) => {
    if (value == null || !Number.isFinite(value)) { connected = false; return ''; }
    const point = `${connected ? 'L' : 'M'}${((seconds - start) / span * 280).toFixed(2)},${(70 - value / max * 60).toFixed(2)}`;
    connected = true; return point;
  }).join(' ');
  return <svg className="metric-chart" viewBox="0 0 280 80" role="img" aria-label={label}><line x1="0" x2="280" y1="70" y2="70" className="timeline-grid"/><path d={path} fill="none" stroke="#177763" strokeWidth="2"/><text x="0" y="12" className="metric-scale">{scale(max)}</text></svg>;
}
function CpuPanels({ cpu, disconnected }: { cpu?: CpuView; disconnected: boolean }) {
  const last = cpu?.history.at(-1);
  const peak = Math.max(100, ...(cpu?.groups ?? []).map(group => group.percent));
  const disabled = <p className="muted">Disabled</p>;
  return <><section className="panel"><h2>CPU total · core %</h2>{cpu ? <><p className="metric-value">{last?.percent != null ? `${last.percent.toFixed(1)}%` : 'Unavailable'}</p>{!!cpu.cores && <p>{cpu.cores} cores</p>}{disconnected && <p className="notice">Connection lost</p>}<Sparkline points={cpu.history.map(sample => ({ seconds: sample.seconds, value: sample.percent }))} label="Total CPU utilization across cores" scale={max => `${max.toFixed(1)}%`}/></> : disabled}</section><section className="panel"><h2>Top CPU consumers</h2>{!cpu ? disabled : cpu.groups.length ? <ol className="cpu-ranking">{cpu.groups.map(group => <li key={group.id}><div><span className="mono" title={`Cgroup ${group.id}`}>{group.name || `Cgroup ${group.id}`}</span><strong>{group.percent.toFixed(1)}%</strong></div><meter min={0} max={peak} value={group.percent} aria-label={`${group.name || `Cgroup ${group.id}`} core utilization`}/></li>)}</ol> : <p className="muted">Unavailable</p>}</section><section className="panel"><h2>Capture health</h2>{cpu ? <dl className="detail-grid"><dt>Snapshot</dt><dd>{cpu.sequence ?? 'Unavailable'}</dd><dt>Dropped batches</dt><dd>{cpu.droppedBatches ?? 'Unavailable'}</dd><dt>Capacity overflow</dt><dd>{cpu.capacityOverflow ?? 'Unavailable'}</dd></dl> : disabled}</section></>;
}
function FaultPanel({ faults }: { faults: FaultView }) {
  return <section className="panel"><h2>Faults</h2><p>{faults.connected}/{faults.sources} sources connected{faults.stale && <> · Waiting for snapshot</>}</p>{faults.conditions.length ? <ul className="fault-list">{faults.conditions.map(condition => <li key={condition.key}><strong>{condition.code}</strong> <span className="mono">{condition.component}{condition.resource && `/${condition.resource}`}</span> · {condition.health === Health.HEALTH_ACTIVE ? 'Active' : 'Recovering'}</li>)}</ul> : <p className="muted">No active faults</p>}</section>;
}
/** Recording metrics come from Status; CPU and fault panels from frames relayed in this session. */
export function RunMetrics({ samples = [], live, historical = false, disconnected = false, incomplete = false, recordingEnabled = true }: { recordingEnabled?: boolean; samples?: RunMetric[]; live?: LiveView; historical?: boolean; disconnected?: boolean; incomplete?: boolean }) {
  const last = samples.at(-1);
  return <><section className="panel"><h2>MCAP recording</h2>{recordingEnabled ? <><p className="metric-value">{bytesLabel(last?.recordingBytes)}</p><p>{last?.writeBytesPerSecond != null ? `${bytesLabel(last.writeBytesPerSecond)}/s` : 'Unavailable'}</p><Sparkline points={samples.map(sample => ({ seconds: Number(sample.elapsedNs) / 1e9, value: sample.writeBytesPerSecond }))} label="MCAP write rate" scale={max => `${bytesLabel(max)}/s`}/></> : <p className="muted">Disabled</p>}</section><CpuPanels cpu={live?.cpu} disconnected={disconnected && !historical}/>{live?.faults && <FaultPanel faults={live.faults}/>}{(incomplete || !!live?.gaps) && <p className="notice">Metrics incomplete</p>}</>;
}
