import { useMemo } from 'react';
import type { RunMetric, Snapshot } from './api';
import { topCpuGroups } from './cpu';

export const bytesLabel = (value: string | number | null | undefined) => {
  if (value == null) return 'Unavailable';
  const bytes = Number(value);
  if (!Number.isFinite(bytes) || bytes < 0) return 'Unavailable';
  const unit = bytes >= 1024 ** 3 ? 3 : bytes >= 1024 ** 2 ? 2 : bytes >= 1024 ? 1 : 0;
  return `${(bytes / 1024 ** unit).toFixed(unit ? 1 : 0)} ${['B', 'KiB', 'MiB', 'GiB'][unit]}`;
};
function Sparkline({ samples, field, label }: { samples: RunMetric[]; field: 'cpuPercent' | 'writeBytesPerSecond'; label: string }) {
  const values = samples.map(sample => sample[field]);
  const max = Math.max(1, ...values.filter((v): v is number => v != null && Number.isFinite(v)));
  const start = Number(samples[0]?.elapsedNs ?? 0) / 1e9;
  const span = Math.max(1, Number(samples.at(-1)?.elapsedNs ?? 0) / 1e9 - start);
  let connected = false;
  const path = samples.map(sample => {
    const value = sample[field];
    if (value == null || !Number.isFinite(value)) { connected = false; return ''; }
    const point = `${connected ? 'L' : 'M'}${((Number(sample.elapsedNs) / 1e9 - start) / span * 280).toFixed(2)},${(70 - value / max * 60).toFixed(2)}`;
    connected = true; return point;
  }).join(' ');
  return <svg className="metric-chart" viewBox="0 0 280 80" role="img" aria-label={label}><line x1="0" x2="280" y1="70" y2="70" className="timeline-grid"/><path d={path} fill="none" stroke="#177763" strokeWidth="2"/><text x="0" y="12" className="metric-scale">{field === 'cpuPercent' ? `${max.toFixed(1)}%` : `${bytesLabel(max)}/s`}</text></svg>;
}
export function RunMetrics({ samples = [], cpuSnapshots = [], historical = false, disconnected = false, incomplete = false }: { samples?: RunMetric[]; cpuSnapshots?: Snapshot['cpuSnapshots']; historical?: boolean; disconnected?: boolean; incomplete?: boolean }) {
  const last = samples.at(-1);
  const cpu = [...samples].reverse().find(sample => sample.cpuPercent != null && (historical || BigInt(last?.elapsedNs ?? '0') - BigInt(sample.elapsedNs) <= 500000000n));
  const source = cpuSnapshots.at(-1);
  const groups = useMemo(() => topCpuGroups(cpuSnapshots), [cpuSnapshots]);
  const peak = Math.max(100, ...groups.map(group => group.percent));
  return <><section className="panel"><h2>MCAP recording</h2><p className="metric-value">{bytesLabel(last?.recordingBytes)}</p><p>{last?.writeBytesPerSecond != null ? `${bytesLabel(last.writeBytesPerSecond)}/s` : 'Waiting for write samples'} · {historical ? 'last observed rate' : 'observed write rate'}</p><Sparkline samples={samples} field="writeBytesPerSecond" label="MCAP file growth rate over retained samples"/></section><section className="panel"><h2>CPU total · core %</h2><p className="metric-value">{cpu?.cpuPercent != null ? `${cpu.cpuPercent.toFixed(1)}%` : 'Unavailable'}</p><p>{historical ? 'Last available sample' : disconnected ? 'Connection lost · values frozen' : 'Recorded cgroup total'}{cpu?.cpuCores ? ` · ${cpu.cpuCores} cores` : ''}</p><Sparkline samples={samples} field="cpuPercent" label="Total recorded cgroup CPU utilization across cores over retained samples"/></section><section className="panel"><h2>Top CPU groups</h2>{groups.length ? <ol className="cpu-ranking">{groups.map(group => <li key={group.id}><div><span className="mono">Cgroup {group.id}</span><strong>{group.percent.toFixed(1)}%</strong></div><meter min={0} max={peak} value={group.percent} aria-label={`Cgroup ${group.id} core utilization`}/></li>)}</ol> : <p className="muted">No CPU group samples available.</p>}</section><section className="panel"><h2>Capture health</h2><dl className="detail-grid"><dt>Snapshot</dt><dd>{source?.sequence?.toString() ?? 'Unavailable'}</dd><dt>Dropped batches</dt><dd>{source?.droppedBatches?.toString() ?? 'Unavailable'}</dd><dt>Capacity overflow</dt><dd>{source?.capacityOverflow?.toString() ?? 'Unavailable'}</dd></dl></section>{incomplete && <p className="notice">Some metric samples are unavailable or incomplete.</p>}</>;
}
