import { useEffect, useState } from "react";
export interface Resources {
  rss_bytes: number | null;
  peak_rss_bytes: number | null;
  capture: {
    used: number;
    capacity: number;
    ready: number;
    reserved_bytes: number;
  };
  encoded: { used: number; capacity: number; reserved_bytes: number };
  replay: {
    bytes: number;
    byte_capacity: number;
    batches: number;
    batch_capacity: number;
  };
  live: {
    clients: number;
    pending_batches: number;
    capacity: number;
    total_bytes: number;
    total_batches: number;
  };
  mailboxes: { name: string; used: number; capacity: number }[];
  registered_threads: number;
}
const bytes = (n: number | null) =>
  n === null
    ? "Unavailable"
    : n < 1024
      ? `${n} B`
      : n < 1048576
        ? `${(n / 1024).toFixed(1)} KiB`
        : `${(n / 1048576).toFixed(1)} MiB`;
function Gauge({
  label,
  used,
  capacity,
  detail,
}: {
  label: string;
  used: number;
  capacity: number;
  detail: string;
}) {
  const fraction = capacity ? Math.min(1, used / capacity) : 0;
  return (
    <div className="resource-gauge">
      <div>
        <span>{label}</span>
        <b>
          {used.toLocaleString()} / {capacity.toLocaleString()}
        </b>
      </div>
      <div
        className="resource-track"
        role="meter"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={capacity}
        aria-valuenow={used}
      >
        <i
          style={{
            width: `${fraction * 100}%`,
            background: fraction > 0.8 ? "#f8c76a" : "#64d9b2",
          }}
        />
      </div>
      <small>{detail}</small>
    </div>
  );
}
function Trend({
  values,
  label,
  color,
}: {
  values: number[];
  label: string;
  color: string;
}) {
  const max = Math.max(1, ...values);
  const points = values
    .map((n, i) => `${(i * 240) / 59},${45 - (n / max) * 40}`)
    .join(" ");
  return (
    <svg
      className="resource-trend"
      viewBox="0 0 240 50"
      role="img"
      aria-label={label}
    >
      <path d="M0 45H240" stroke="#365164" />
      <polyline points={points} fill="none" stroke={color} strokeWidth="2" />
    </svg>
  );
}
type Sample = {
  time: number;
  rss: number | null;
  total: number;
  rate: number | null;
  capture: number;
  encoded: number;
};
export function ResourcePanel({
  resources: r,
  spans,
  events,
}: {
  resources: Resources;
  spans: number;
  events: number;
}) {
  const [history, setHistory] = useState<Sample[]>([]);
  useEffect(() => {
    const time = performance.now();
    setHistory((old) => {
      const last = old.at(-1);
      const reset = last && r.live.total_bytes < last.total;
      const rate =
        last && !reset
          ? (r.live.total_bytes - last.total) /
            Math.max(0.001, (time - last.time) / 1000)
          : null;
      return [
        ...(reset ? [] : old),
        {
          time,
          rss: r.rss_bytes,
          total: r.live.total_bytes,
          rate,
          capture: r.capture.used,
          encoded: r.encoded.used,
        },
      ]
        .filter((s) => s.time >= time - 60000)
        .slice(-60);
    });
  }, [r]);
  const rate = history.at(-1)?.rate ?? null;
  const reserved = r.capture.reserved_bytes + r.encoded.reserved_bytes;
  return (
    <section
      className="panel resources-panel"
      aria-label="Memory and buffer usage"
    >
      <div className="panel-heading">
        <div>
          <h2>Memory & buffers</h2>
          <p>Server process + browser retention · sampled every second</p>
        </div>
        <span className="zoom-hint">
          {r.live.clients} live client{r.live.clients === 1 ? "" : "s"}
        </span>
      </div>
      <div className="resource-summary">
        <div>
          <span className="metric-label">PROCESS RSS</span>
          <strong>{bytes(r.rss_bytes)}</strong>
          <Trend
            values={history.map((s) => s.rss ?? 0)}
            label="Server RSS, last 60 samples, auto-scaled"
            color="#b59bfa"
          />
          <small>Process high-water: {bytes(r.peak_rss_bytes)}</small>
        </div>
        <div>
          <span className="metric-label">RESERVED POOL PAYLOAD</span>
          <strong>{bytes(reserved)}</strong>
          <div className="pool-budget">
            <i
              style={{
                width: `${(r.capture.reserved_bytes / reserved) * 100}%`,
              }}
            />
            <i />
          </div>
          <small>
            <i className="capture-key" /> Capture{" "}
            {bytes(r.capture.reserved_bytes)}
            <br />
            <i className="encoded-key" /> Encoded{" "}
            {bytes(r.encoded.reserved_bytes)}
          </small>
        </div>
        <div>
          <span className="metric-label">ENCODED THROUGHPUT</span>
          <strong>
            {rate === null ? "Collecting…" : `${bytes(Math.round(rate))}/s`}
          </strong>
          <Trend
            values={history.map((s) => s.rate ?? 0)}
            label="Encoded batch bytes per second, last 60 samples, auto-scaled"
            color="#74a9e8"
          />
          <small>
            {r.live.total_batches.toLocaleString()} batches ·{" "}
            {bytes(r.live.total_bytes)} total
          </small>
        </div>
      </div>
      <div className="resource-gauges">
        <Gauge
          label="Capture blocks in use"
          used={r.capture.used}
          capacity={r.capture.capacity}
          detail={`${r.capture.ready} queued for encoding · observed peak ${Math.max(r.capture.used, ...history.map((s) => s.capture))}`}
        />
        <Gauge
          label="Encoded buffers leased"
          used={r.encoded.used}
          capacity={r.encoded.capacity}
          detail={`Shared across sinks · observed peak ${Math.max(r.encoded.used, ...history.map((s) => s.encoded))}`}
        />
        <Gauge
          label="Replay bytes"
          used={r.replay.bytes}
          capacity={r.replay.byte_capacity}
          detail={`${bytes(r.replay.bytes)} · ${r.replay.batches}/${r.replay.batch_capacity} batches; either limit evicts oldest`}
        />
        <Gauge
          label="Live broadcast backlog"
          used={r.live.pending_batches}
          capacity={r.live.capacity}
          detail="Shared batch references · slow clients can cause drops"
        />
        {r.mailboxes.map((q) => (
          <Gauge
            key={q.name}
            label={`${q.name} mailbox`}
            used={q.used}
            capacity={q.capacity}
            detail="Pending messages · bounded try_send"
          />
        ))}
      </div>
      <div className="resource-notes">
        <span>
          Browser retained: <b>{spans}/2,000 spans</b> ·{" "}
          <b>{events}/600 events</b> (counts; JS heap size is unmeasured).
        </span>
        <span>
          RSS covers the whole server. Reserved capacity can exceed RSS while
          pages remain untouched. Pool payload excludes metadata, stacks,
          registry and sink allocations. Replay/broadcast share bytes; avoid
          summing them. Sampled peaks can miss short bursts. Throughput measures
          encoded bytes; memory bandwidth and allocation rate are unmeasured.
        </span>
      </div>
    </section>
  );
}
