import type { RunMetric } from './api';

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
  return <svg className="metric-chart" viewBox="0 0 280 80" role="img" aria-label={label}><line x1="0" x2="280" y1="70" y2="70" className="timeline-grid"/><path d={path} fill="none" stroke="#177763" strokeWidth="2"/><text x="0" y="12" className="metric-scale">{max.toFixed(1)} {field === 'cpuPercent' ? '%' : 'B/s'}</text></svg>;
}
export function RunMetrics({ samples = [], historical = false, disconnected = false, incomplete = false }: { samples?: RunMetric[]; historical?: boolean; disconnected?: boolean; incomplete?: boolean }) {
  const last = samples.at(-1);
  const cpu = historical ? [...samples].reverse().find(sample => sample.cpuPercent != null) : last;
  return <><section className="panel"><h2>MCAP recording</h2><p className="metric-value">{bytesLabel(last?.recordingBytes)}</p><p>{last?.writeBytesPerSecond != null ? `${bytesLabel(last.writeBytesPerSecond)}/s` : 'Waiting for write samples'} · {historical ? 'last observed rate' : 'observed write rate'}</p><Sparkline samples={samples} field="writeBytesPerSecond" label="MCAP file growth rate over retained samples"/><p className="muted">File growth at the recorder boundary; durability is verified after Stop.</p></section><section className="panel"><h2>CPU across cores</h2><p className="metric-value">{cpu?.cpuPercent != null ? `${cpu.cpuPercent.toFixed(1)}%` : 'Unavailable'}</p><p>{historical ? 'Last available sample' : disconnected ? 'Connection lost · values frozen' : 'Recorded cgroup total'}{cpu?.cpuCores ? ` · ${cpu.cpuCores} cores` : ''}</p><Sparkline samples={samples} field="cpuPercent" label="Total recorded cgroup CPU utilization across cores over retained samples"/><p className="muted">100% equals one fully used core. Inclusive ancestors count once; root-unavailable and virtual kernel rows are excluded.</p></section>{incomplete && <p className="notice">Some metric samples are unavailable or incomplete.</p>}</>;
}
