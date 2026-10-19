import { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import uPlot from 'uplot';
import 'uplot/dist/uPlot.min.css';
import './style.css';
import { clockSample, browserDelay, type ClockSample } from './latency';
import { StreamDecoder } from './protocol';
import { TopCpuStreams, MAX_GROUPS, MAX_CPUS, KERNEL_SERIES, WINDOW, type Point, type Snapshot } from './stream';

function bytesLabel(bytes: number): string {
  const units = ['B', 'KiB', 'MiB', 'GiB'];
  let unit = 0;
  while (bytes >= 1024 && unit < units.length - 1) { bytes /= 1024; unit++; }
  return `${bytes.toFixed(unit ? 2 : 0)} ${units[unit]}`;
}
function milliseconds(value: number | null | undefined): string {
  return value == null ? '—' : `${Number(value).toFixed(2)} ms`;
}
function color(id: string) {
  if (id === KERNEL_SERIES) return '#111827';
  let hash = 0;
  for (const char of id) hash = (Math.imul(hash, 31) + char.charCodeAt(0)) | 0;
  return `hsl(${((hash >>> 0) * 137.508) % 360} 65% 43%)`;
}
const cpuLabel = (cpu: string) => cpu === '4294967295' ? 'All CPUs (aggregate)' : `CPU ${cpu}`;
type Group = { id: string; label: string };
type Frame = { now: number; charts: { cpu: string; rows: Point[]; groups: Group[] }[]; cpus: string[]; gaps: string; points: number; age?: number };

type LegendBranch = { name: string; path: string; children: Map<string, LegendBranch>; groups: Group[] };
function CgroupLegend({ groups, point }: { groups: Group[]; point?: Point }) {
  const root: LegendBranch = { name: '/', path: '/', children: new Map(), groups: [] };
  const unresolved: Group[] = [];
  for (const group of groups) {
    if (group.id === KERNEL_SERIES) continue;
    if (!group.label.startsWith('/')) { unresolved.push(group); continue; }
    let node = root;
    for (const part of group.label.split('/').filter(Boolean)) {
      if (!node.children.has(part)) node.children.set(part, {
        name: part, path: `${node.path === '/' ? '' : node.path}/${part}`, children: new Map(), groups: [],
      });
      node = node.children.get(part)!;
    }
    node.groups.push(group);
  }
  const value = (group: Group) => {
    const usage = point?.values.get(group.id);
    return <span className="tree-value">{usage == null ? '—' : `${usage.toFixed(1)}%`}</span>;
  };
  const marker = (group: Group) => <span className={`tree-swatch ${group.id === KERNEL_SERIES ? 'dashed' : ''}`} style={{ borderColor: color(group.id) }}/>;
  function branch(node: LegendBranch): React.ReactNode {
    return <li key={node.path}>
      <div className={`tree-row ${node.groups.length ? 'measured' : 'ancestor'}`} title={node.path}>
        {node.groups.length === 1 ? marker(node.groups[0]) : <span className="tree-folder" aria-hidden="true">▸</span>}
        <span className="tree-name">{node.name}</span>
        {node.groups.length === 1 && value(node.groups[0])}
      </div>
      {node.groups.length > 1 && <ul>{node.groups.map(group => <li key={group.id}><div className="tree-row">{marker(group)}<span className="tree-name">ID {group.id}</span>{value(group)}</div></li>)}</ul>}
      {!!node.children.size && <ul>{[...node.children.values()].sort((a, b) => a.name.localeCompare(b.name)).map(branch)}</ul>}
    </li>;
  }
  return <div className="cgroup-legend" aria-label="Legend by cgroup path">
    <div className="legend-caption">Cgroup path <span>{point ? new Date(point.time * 1000).toLocaleTimeString() : 'Waiting'}</span></div>
    <ul className="cgroup-tree">{branch(root)}</ul>
    {unresolved.length > 0 && <div className="legend-separate"><div className="legend-caption">Unresolved cgroup paths</div>{unresolved.sort((a,b) => a.id.localeCompare(b.id)).map(group => <div className="tree-row" key={group.id}>{marker(group)}<span className="tree-name">{group.label}</span>{value(group)}</div>)}</div>}
    {groups.filter(g => g.id === KERNEL_SERIES).map(group => <div className="legend-separate" key={group.id}><div className="legend-caption">Virtual cgroup · overlaps real cgroups</div><div className="tree-row">{marker(group)}<span className="tree-name">{group.label}</span>{value(group)}</div></div>)}
  </div>;
}

function CpuChart({ cpu, groups, frame }: { cpu: string; groups: Group[]; frame: Frame }) {
  const host = useRef<HTMLDivElement>(null);
  const chart = useRef<uPlot | undefined>(undefined);
  const signature = JSON.stringify(groups);
  const [cursorIndex, setCursorIndex] = useState<number | null>(null);
  useEffect(() => {
    const element = host.current!;
    const plot = new uPlot({
      width: Math.max(250, element.clientWidth), height: 250,
      legend: { show: false },
      hooks: { setCursor: [plot => setCursorIndex(plot.cursor.idx ?? null)] },
      cursor: { drag: { x: false, y: false } },
      scales: { x: { time: true }, y: { range: (_u, _min, max) => [0, Math.max(100, (max || 0) * 1.1)] } },
      axes: [{ stroke: '#64748b', grid: { stroke: '#e8edf3' } }, { stroke: '#64748b', values: (_u, ticks) => ticks.map(v => `${v}%`), grid: { stroke: '#e8edf3' } }],
      series: [{}, ...groups.map(group => ({ label: group.label, stroke: color(group.id), width: group.id === KERNEL_SERIES ? 2.5 : 2, dash: group.id === KERNEL_SERIES ? [6, 4] : [], spanGaps: false, value: (_u: uPlot, value: number) => value == null ? '—' : `${value.toFixed(1)}%` }))],
    }, [[], ...groups.map(() => [])], element);
    chart.current = plot;
    setCursorIndex(null);
    const resize = new ResizeObserver(() => plot.setSize({ width: Math.max(250, element.clientWidth), height: 250 }));
    resize.observe(element);
    return () => { resize.disconnect(); plot.destroy(); chart.current = undefined; };
  }, [signature]);
  useEffect(() => {
    const plot = chart.current;
    if (!plot) return;
    const rows = frame.charts.find(chart => chart.cpu === cpu)?.rows ?? [];
    plot.setData([rows.map(p => p.time), ...groups.map(group => rows.map(p => p.values.get(group.id) ?? null))], false);
    plot.setScale('x', { min: frame.now - WINDOW, max: frame.now });
  }, [frame, cpu, signature]);
  const rows = frame.charts.find(item => item.cpu === cpu)?.rows ?? [];
  const point = rows[cursorIndex ?? rows.length - 1];
  return <section className="chart-card cpu-card"><h2>{cpuLabel(cpu)} <small>Top {groups.filter(g => g.id !== KERNEL_SERIES).length} active cgroups</small></h2><div ref={host}/><CgroupLegend groups={groups} point={point}/>{!groups.length && <p className="note">No measured activity in the last 5 seconds.</p>}</section>;
}

function App() {
  const model = useRef(new TopCpuStreams());
  const names = useRef(new Map<string, string>());
  const kernelEnabled = useRef(false);
  const [status, setStatus] = useState('Connecting');
  const [latest, setLatest] = useState<Snapshot>();
  const [browserLatency, setBrowserLatency] = useState<number | null>(null);
  const [clockDetail, setClockDetail] = useState('Calibrating browser/server clocks…');
  const [frame, setFrame] = useState<Frame>({ now: Date.now() / 1000, charts: [], cpus: [], gaps: '0', points: 0 });
  useEffect(() => {
    const decoder = new StreamDecoder();
    let lastArrival: number | undefined;
    let session: string | undefined;
    const origin = Date.now() / 1000 - performance.now() / 1000;
    const now = () => origin + performance.now() / 1000;
    let source: WebSocket;
    let reconnect: number | undefined;
    let disposed = false;
    let clock: ClockSample | undefined;
    const clockAbort = new AbortController();
    async function calibrate() {
      let best: ClockSample | undefined;
      for (let i = 0; i < 3 && !disposed; i++) {
        try {
          const start = performance.now();
          const response = await fetch('/clock', { cache: 'no-store', signal: AbortSignal.any([clockAbort.signal, AbortSignal.timeout(3000)]) });
          if (!response.ok) throw new Error('Clock endpoint unavailable');
          const timestamp = await response.text();
          const sample = clockSample(timestamp, start, performance.now());
          if (!best || sample.uncertaintyMs < best.uncertaintyMs) best = sample;
        } catch { /* Try another bounded sample; retain recent calibration. */ }
      }
      if (disposed) return;
      if (best) clock = best;
      if (!clock || performance.now() - clock.midpointMs > 90_000) {
        clock = undefined;
        setClockDetail('Clock calibration unavailable');
      }
    }
    void calibrate();
    const clockTimer = window.setInterval(() => { void calibrate(); }, 30000);
    function connect() {
      const url = new URL('/stream', window.location.href);
      url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
      source = new WebSocket(url);
      source.binaryType = 'arraybuffer';
      source.onopen = () => setStatus('Waiting for DORA');
      source.onerror = () => setStatus('Disconnected · retrying');
      source.onclose = () => {
        if (!disposed) { setStatus('Disconnected · retrying'); reconnect = window.setTimeout(connect, 1000); }
      };
      source.onmessage = event => {
      const receivedAtMs = performance.now(); // Timestamp callback entry, before decoding.
      try {
        const s = decoder.receive(new Uint8Array(event.data));
        if (!s) return;
        if (!Array.isArray(s.records) || !Array.isArray(s.dictionary)) throw new Error('Invalid snapshot');
        if (session !== s.session_id) { names.current.clear(); session = s.session_id; }
        for (const d of s.dictionary) names.current.set(d.id, d.path === null ? `Cgroup · ${d.id}` : `/${d.path.replace(/^\/+/, '')}`);
        kernelEnabled.current = s.task_collection_enabled === true;
        model.current.ingest(s, now());
        lastArrival = now();
        const latencyMs = clock ? browserDelay(s.capture_unix_ns, receivedAtMs, clock) : null;
        setBrowserLatency(latencyMs);
        setClockDetail(!clock ? 'Calibrating / clock unavailable' : latencyMs === null
          ? 'Clock changed or calibration expired'
          : `Clock estimate ±${clock.uncertaintyMs.toFixed(1)} ms · before decode/render`);
        setLatest(s);
        setStatus('Live');
      } catch { setStatus('Invalid snapshot received'); }
      };
    }
    connect();
    const timer = window.setInterval(() => {
      const current = now();
      model.current.refresh(current);
      const charts = [...model.current.buffers].sort(([a], [b]) => Number(a) - Number(b)).map(([cpu, buffer]) => ({
        cpu, rows: buffer.rows(),
        // Stable series ordering/color while ranking membership is unchanged.
        groups: [
          ...(kernelEnabled.current ? [{ id: KERNEL_SERIES, label: 'virtual/kernel · PF_KTHREAD' }] : []),
          ...[...(model.current.top.get(cpu) ?? [])].sort().map(id => ({ id, label: names.current.get(id) ?? `Cgroup · ${id}` })),
        ],
      }));
      setFrame({ now: current, charts,
        cpus: charts.map(c => c.cpu),
        gaps: model.current.gaps.toString(),
        points: Math.max(0, ...charts.map(c => c.rows.length)),
        age: lastArrival === undefined ? undefined : current - lastArrival,
      });
    }, 100);
    return () => { disposed = true; clockAbort.abort(); clearInterval(clockTimer); clearTimeout(reconnect); source.close(); clearInterval(timer); };
  }, []);
  const ingress = latest?.viewer_ingress;
  const fresh = status === 'Live' && (frame.age ?? Infinity) < 2;
  return <main>
    <header><div><p className="eyebrow">DORAMCAP / LIVE TELEMETRY</p><h1>CPU activity by cgroup</h1><p className="subtitle">One live plot per CPU. Each line is a cgroup’s usage on that CPU.</p></div><span className={`status ${status === 'Live' && (frame.age ?? 0) < 2 ? 'live' : ''}`} role="status">{status === 'Live' && (frame.age ?? 0) >= 2 ? 'Stale · waiting for data' : status}</span></header>
    <div className="pipeline"><span>system-monitor</span><b>→</b><span>DORA snapshots</span><b>→</b><span>cpu-viewer / binary WS</span><b>→</b><span>React</span></div>
    <section className="metrics">
      <div><small>DATA SOURCE</small><strong>{latest?.source ?? 'Waiting'}</strong></div>
      <div><small>CPU PLOTS</small><strong>{frame.cpus.length}</strong></div>
      <div><small>STREAM GAPS</small><strong>{frame.gaps}</strong></div>
      <div><small>ROLLING WINDOW</small><strong>30 s <em> / {frame.points} points</em></strong></div>
    </section>
    <section className="metrics ingress-metrics" aria-label="DORA ingress telemetry">
      <div title="Original snapshot and dictionary payload bytes received by cpu-viewer, measured over approximately one second. Excludes Dora transport overhead and WebSocket/browser traffic."><small>DORA INGRESS</small><strong>{fresh && ingress?.bytes_per_second != null ? `${bytesLabel(Number(ingress.bytes_per_second))}/s` : '—'}</strong><p className="metric-detail">{ingress ? `${bytesLabel(Number(ingress.total_bytes))} received since viewer start` : 'Waiting for measurements'}</p></div>
      <div><small>SNAPSHOT RATE</small><strong>{fresh && ingress?.messages_per_second != null ? `${Number(ingress.messages_per_second).toFixed(1)} /s` : '—'}</strong><p className="metric-detail">{ingress?.total_messages ?? '0'} snapshots received</p></div>
      <div title="Producer capture timestamp to viewer subscriber dequeue. Includes collection, publication, queueing and DORA delivery; excludes WebSocket delivery and browser rendering. Uses the shared host wall clock."><small>CAPTURE → BACKEND</small><strong>{milliseconds(fresh ? ingress?.capture_to_receive_ms : null)}</strong><p className="metric-detail">Latest · mean {milliseconds(fresh ? ingress?.mean_capture_to_receive_ms : null)} over ~1 s</p></div>
      <div title="Producer capture timestamp to browser WebSocket callback entry, before decoding. Includes backend processing, WebSocket delivery and browser scheduling. Uses a periodically calibrated server clock and browser monotonic time. Approximate one-way latency; network asymmetry affects the estimate. Excludes decoding and rendering."><small>CAPTURE → BROWSER ≈</small><strong>{milliseconds(fresh ? browserLatency : null)}</strong><p className="metric-detail">{clockDetail}</p></div>
    </section>
    <p className="note">Ingress counts DORA snapshot payloads, including cgroup counters, virtual kernel totals, and the low-rate dictionary. Both latencies start at capture. Backend ends at DORA dequeue; browser includes onward delivery and callback scheduling, before decode/render. Browser timing uses the fastest of three clock probes every 30 seconds; the estimate assumes roughly symmetric delivery and shows half the calibration round-trip as uncertainty; values are not additive.{ingress && BigInt(ingress.invalid_timestamps) > 0n ? ` ${ingress.invalid_timestamps} invalid timestamps excluded.` : ''}</p>
    <p className="note">Automatically showing up to {MAX_GROUPS} busiest cgroups per CPU, ranked by the last 5 seconds and updated once per second. 100% = one occupied core. Counters include descendants; lines are not stacked or summed. Up to {MAX_CPUS} CPUs · 10 Hz refresh.</p>
    <p className="note kernel-status" role="status">Kernel-thread collector: <strong>{latest?.task_collection_enabled ? 'enabled (experimental)' : 'disabled'}</strong>. The dashed line is a reporting aggregate of PF_KTHREAD runtime, not a real cgroup or total kernel/IRQ/idle time. It may overlap cgroup lines; do not add them together.</p>
    {latest?.task_collection_enabled && <p className="note">Kernel-only task map capacity: {latest.task_capacity ?? '—'} · Lost task events: {latest.task_losses?.dropped_events ?? '0'} · Lost runtime: {(Number(latest.task_losses?.dropped_runtime_ns ?? '0') / 1e6).toFixed(2)} ms · Identity errors: {latest.task_losses?.identity_errors ?? '0'}</p>}
    {latest?.task_collection_enabled && (BigInt(latest.task_losses?.dropped_events ?? '0') > 0n || BigInt(latest.task_losses?.identity_errors ?? '0') > 0n) && <p role="alert" className="loss-warning">Task collection is incomplete. The fixed task map can fill over time; restart the dataflow to begin a new collection session.</p>}
    <div className="cpu-grid">{frame.charts.map(chart => <CpuChart key={chart.cpu} cpu={chart.cpu} groups={chart.groups} frame={frame}/>)}</div>
    {!frame.cpus.length && <p className="note">Waiting for CPU samples…</p>}
    <footer><span>Sequence: {latest?.sequence ?? '—'} · Collection drops: {latest?.dropped_batches ?? '0'} · Record overflow: {latest?.capacity_overflow ?? '0'}</span><span>{latest ? `Captured ${new Date(Number(BigInt(latest.capture_unix_ns) / 1_000_000n)).toLocaleTimeString()}` : 'Waiting for the first published snapshot'}</span></footer>
  </main>;
}
createRoot(document.getElementById('root')!).render(<App/>);
