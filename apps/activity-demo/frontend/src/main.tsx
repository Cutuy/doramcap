import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { TraceStore, TraceSpan, DemoStatus, phaseColor, depth } from "./model";
import "./style.css";
import { zoomTimeline } from "./timeline";
import { ResourcePanel } from "./Resources";
import { SleepControls } from "./SleepControls";
const WORKERS = ["Aster", "Birch", "Cedar", "Dune"];
const COLORS = ["#64d9b2", "#74a9e8", "#b59bfa", "#f8c76a"];
function App() {
  const store = useRef(new TraceStore());
  const [tick, setTick] = useState(0);
  const [connection, setConnection] = useState("Connecting");
  const [error, setError] = useState("");
  const [status, setStatus] = useState<DemoStatus>();
  const [selected, setSelected] = useState("");
  const [windowSeconds, setWindow] = useState(12);
  const [frozen, setFrozen] = useState<number | null>(null);
  const timeline = useRef<SVGSVGElement>(null);
  const viewport = useRef({ seconds: windowSeconds, frozen });
  viewport.current = { seconds: windowSeconds, frozen };
  useEffect(() => {
    const svg = timeline.current;
    if (!svg) return;
    function wheel(event: WheelEvent) {
      const matrix = svg!.getScreenCTM();
      if (!matrix) return;
      const point = svg!.createSVGPoint();
      point.x = event.clientX;
      point.y = event.clientY;
      const local = point.matrixTransform(matrix.inverse());
      if (local.x < 170 || local.x > 1060) return;
      const delta =
        (event.deltaY || event.deltaX) *
        (event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? 495 : 1);
      if (!delta) return;
      event.preventDefault();
      const current = viewport.current;
      const right = Math.max(
        current.seconds,
        current.frozen ?? store.current.now(),
      );
      const next = zoomTimeline(
        current.seconds,
        right,
        (local.x - 170) / 890,
        delta,
      );
      viewport.current = { seconds: next.seconds, frozen: next.right };
      setWindow(next.seconds);
      setFrozen(next.right);
    }
    svg.addEventListener("wheel", wheel, { passive: false });
    return () => svg.removeEventListener("wheel", wheel);
  }, []);
  const [busy, setBusy] = useState(false);
  const [transportGaps, setTransportGaps] = useState(0);
  useEffect(() => {
    let closed = false;
    let socket: WebSocket;
    let retry: ReturnType<typeof setTimeout>;
    function connect() {
      if (closed) return;
      setConnection("Connecting");
      socket = new WebSocket(
        `${location.protocol === "https:" ? "wss:" : "ws:"}//${location.host}/stream`,
      );
      socket.binaryType = "arraybuffer";
      socket.onopen = () => {
        setConnection("Live");
        setError("");
      };
      socket.onmessage = (e) => {
        try {
          if (typeof e.data === "string") {
            const m = JSON.parse(e.data);
            if (m.type === "gap")
              setTransportGaps((v) => v + Number(m.batches));
          } else store.current.ingest(new Uint8Array(e.data));
        } catch (e) {
          setError(String(e));
        }
      };
      socket.onerror = () => setConnection("Reconnecting");
      socket.onclose = () => {
        if (!closed) {
          setConnection("Reconnecting");
          retry = setTimeout(connect, 1000);
        }
      };
    }
    connect();
    const redraw = setInterval(() => setTick((v) => v + 1), 100);
    return () => {
      closed = true;
      clearTimeout(retry);
      clearInterval(redraw);
      socket?.close();
    };
  }, []);
  useEffect(() => {
    let alive = true;
    async function poll() {
      try {
        const r = await fetch("/api/status");
        if (!r.ok) throw new Error("Server unavailable");
        const s = await r.json();
        if (alive) setStatus(s);
      } catch (e) {
        if (alive) setError(String(e));
      }
    }
    void poll();
    const interval = setInterval(poll, 1000);
    return () => {
      alive = false;
      clearInterval(interval);
    };
  }, []);
  const s = store.current;
  const end = frozen ?? s.now(),
    start = Math.max(0, end - windowSeconds),
    right = Math.max(windowSeconds, end);
  const spans = [...s.spans.values()];
  const visible = spans.filter(
    (p) => (p.end ?? right) >= start && p.start <= right,
  );
  const focus = s.spans.get(selected);
  const selectedRoot = focus?.root;
  const messages = [...s.events]
    .reverse()
    .filter(
      (e) => e.fields.phase === "publish" || e.fields.phase === "receive",
    );
  async function action(path: string, body?: object) {
    setBusy(true);
    try {
      const r = await fetch(path, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: body ? JSON.stringify(body) : undefined,
      });
      if (!r.ok) throw new Error("Control request failed");
      if (body && status) setStatus({ ...status, ...body });
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  function choose(p: TraceSpan) {
    setSelected(p.id);
  }
  const x = (time: number) =>
    170 + ((time - (right - windowSeconds)) / windowSeconds) * 890;
  const y = (p: TraceSpan) =>
    56 + WORKERS.indexOf(p.worker) * 105 + depth(p, s.spans) * 19;
  const totalLoss =
    (status?.capture_drops ?? s.producerLoss) +
    (status?.mailbox_drops ?? 0) +
    s.gaps;
  void tick;
  return (
    <main>
      <header>
        <div className="brand">
          <span className="brand-icon">⠿</span>
          <div>
            <div className="eyebrow">DORAMCAP / ACTIVITY LAB</div>
            <h1>
              Threads in conversation<span>.</span>
            </h1>
          </div>
        </div>
        <div className="controls">
          <span
            className={`connection ${connection === "Live" ? "online" : ""}`}
          >
            <i />
            {connection}
          </span>
          <button
            disabled={busy || !status}
            onClick={() => action("/api/control", { paused: !status?.paused })}
          >
            {status?.paused ? "▶ Resume publishing" : "Ⅱ Pause publishing"}
          </button>
          <button
            className="primary"
            disabled={busy || !status}
            onClick={() => action("/api/greet")}
          >
            Send hello <span>↗</span>
          </button>
        </div>
      </header>
      {error && (
        <div className="notice" role="alert">
          {error}
        </div>
      )}
      <section className="metrics">
        <div>
          <span className="metric-label">WORKER THREADS</span>
          <strong>
            04 <small>independent lanes</small>
          </strong>
        </div>
        <div>
          <span className="metric-label">PUBLISHED / DELIVERED</span>
          <strong>
            {status?.published ?? 0}
            <b>/</b>
            {status?.received ?? 0}
            <small>broadcast + reply</small>
          </strong>
        </div>
        <div>
          <span className="metric-label">TRACE RECORDS</span>
          <strong>
            {s.received.toLocaleString()}
            <small>live + recent history</small>
          </strong>
        </div>
        <div>
          <span className="metric-label">LOSS SIGNALS</span>
          <strong className={totalLoss ? "warn" : "mint"}>
            {totalLoss}
            <small>
              {transportGaps
                ? `${transportGaps} transport skips`
                : "capture · mailbox · batch gaps"}
            </small>
          </strong>
        </div>
      </section>
      {status && (
        <SleepControls
          initial={{ ...status.sleep, frequency_hz: status.frequency_hz }}
        />
      )}
      <div className="workspace">
        <section className="panel timeline-panel">
          <div className="panel-heading">
            <div>
              <h2>Activity timeline</h2>
              <p>Publish → receive links across real OS threads</p>
            </div>
            <div className="timeline-controls">
              <span
                className="zoom-hint"
                title="Scroll over the plot to zoom around the pointer"
              >
                Scroll to zoom ·{" "}
                {windowSeconds < 1
                  ? `${Math.round(windowSeconds * 1000)} ms`
                  : `${windowSeconds.toFixed(1)} s`}
              </span>
              <button
                className={
                  frozen === null ? "live-toggle active" : "live-toggle"
                }
                onClick={() => setFrozen(frozen === null ? s.now() : null)}
              >
                {frozen === null ? "● Following live" : "▶ Follow live"}
              </button>
            </div>
          </div>
          <div className="timeline-scroll">
            <svg
              ref={timeline}
              className="timeline"
              viewBox="0 0 1100 495"
              role="img"
              aria-label="Live worker activity timeline"
            >
              <defs>
                <marker
                  id="causal-arrow"
                  viewBox="0 0 8 8"
                  refX="7"
                  refY="4"
                  markerWidth="5"
                  markerHeight="5"
                  orient="auto"
                >
                  <path d="M0 0L8 4L0 8" fill="#f8c76a" />
                </marker>
                <clipPath id="plot">
                  <rect x="170" y="30" width="890" height="440" />
                </clipPath>
              </defs>
              {Array.from({ length: 7 }, (_, i) => (
                <g key={i}>
                  <line
                    x1={170 + (i * 890) / 6}
                    y1="30"
                    x2={170 + (i * 890) / 6}
                    y2="470"
                    stroke="#24303c"
                    strokeDasharray="3 6"
                  />
                  <text
                    x={170 + (i * 890) / 6}
                    y="20"
                    textAnchor="middle"
                    className="axis-label"
                  >
                    {(right - windowSeconds + (i * windowSeconds) / 6).toFixed(
                      windowSeconds < 1 ? 3 : windowSeconds < 6 ? 2 : 1,
                    )}
                    s
                  </text>
                </g>
              ))}
              {WORKERS.map((name, i) => {
                const thread = spans.find((p) => p.worker === name);
                return (
                  <g key={name}>
                    <rect
                      x="170"
                      y={40 + i * 105}
                      width="890"
                      height="95"
                      rx="6"
                      fill={i % 2 ? "#101a23" : "#131e28"}
                    />
                    <circle cx="28" cy={66 + i * 105} r="5" fill={COLORS[i]} />
                    <text x="45" y={71 + i * 105} className="worker-name">
                      {name}
                    </text>
                    <text x="45" y={91 + i * 105} className="axis-label">
                      {thread ? `TID ${thread.tid}` : "waiting…"}
                    </text>
                    <text x="45" y={112 + i * 105} className="worker-topic">
                      hello + {name.toLowerCase()}
                    </text>
                  </g>
                );
              })}
              <g clipPath="url(#plot)">
                {visible
                  .filter((p) => p.name === "receive")
                  .map((p) => {
                    const parent = s.spans.get(p.parent);
                    if (!parent || parent.worker === p.worker) return null;
                    const a = x(parent.start),
                      b = x(p.start),
                      ay = y(parent) + 8,
                      by = y(p) + 8;
                    return (
                      <path
                        key={`link-${p.id}`}
                        d={`M${a},${ay} C${a + 20},${ay} ${b - 20},${by} ${b},${by}`}
                        fill="none"
                        stroke="#f8c76a"
                        opacity={
                          selectedRoot
                            ? p.root === selectedRoot
                              ? 0.9
                              : 0.08
                            : 0.27
                        }
                        strokeWidth={p.root === selectedRoot ? 2 : 1.2}
                        markerEnd="url(#causal-arrow)"
                      />
                    );
                  })}
                {visible.map((p) => {
                  const a = x(p.start),
                    width = Math.max(5, x(p.end ?? right) - a);
                  const chosen = selectedRoot === p.root;
                  return (
                    <g
                      key={p.id}
                      tabIndex={0}
                      role="button"
                      aria-label={`${p.worker} ${p.name}`}
                      onClick={() => choose(p)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          choose(p);
                        }
                      }}
                      className="span-target"
                      opacity={selectedRoot && !chosen ? 0.25 : 1}
                    >
                      <title>
                        {p.worker} · {p.name} ·{" "}
                        {p.end
                          ? `${((p.end - p.start) * 1000).toFixed(2)} ms`
                          : "in progress"}
                      </title>
                      <rect
                        x={a}
                        y={y(p)}
                        width={width}
                        height="15"
                        rx="3"
                        fill={phaseColor(p.name)}
                        fillOpacity={p.name === "greeting" ? 0.25 : 0.85}
                        stroke={selected === p.id ? "#fff" : phaseColor(p.name)}
                        strokeWidth={selected === p.id ? 1.5 : 0.4}
                        strokeDasharray={p.partial ? "3 2" : undefined}
                      />
                      {width > 42 && (
                        <text x={a + 6} y={y(p) + 11} className="span-label">
                          {p.name}
                        </text>
                      )}
                    </g>
                  );
                })}
                {s.events
                  .filter(
                    (e) => e.time >= right - windowSeconds && e.time <= right,
                  )
                  .map((e) => (
                    <circle
                      key={e.id}
                      cx={x(e.time)}
                      cy={126 + WORKERS.indexOf(e.worker) * 105}
                      r="2.3"
                      fill={
                        e.fields.phase === "publish" ? "#f8c76a" : "#64d9b2"
                      }
                      opacity={
                        selectedRoot && e.root !== selectedRoot ? 0.12 : 0.65
                      }
                    />
                  ))}
              </g>
              {!spans.length && (
                <text
                  x="615"
                  y="245"
                  textAnchor="middle"
                  className="empty-label"
                >
                  Waiting for worker activity…
                </text>
              )}
            </svg>
          </div>
          <div className="legend">
            {["sleep", "work", "publish", "receive"].map((name) => (
              <span key={name}>
                <i style={{ background: phaseColor(name) }} />
                {name}
              </span>
            ))}
            <span className="legend-flow">↳ message causality</span>
            <span className="legend-hint">
              Select a span to follow its trace
            </span>
          </div>
        </section>
        <aside className="panel inspector">
          <div className="panel-heading">
            <h2>Trace inspector</h2>
            {focus && (
              <button
                className="clear"
                aria-label="Clear selection"
                onClick={() => setSelected("")}
              >
                ×
              </button>
            )}
          </div>
          {focus ? (
            <>
              <div className="selected-name">
                <i style={{ background: phaseColor(focus.name) }} />
                {focus.name}
              </div>
              <div className="inspector-summary">
                <span>
                  {focus.worker} · TID {focus.tid}
                </span>
                <strong>
                  {focus.end
                    ? `${((focus.end - focus.start) * 1000).toFixed(2)} ms`
                    : "In progress"}
                </strong>
              </div>
              <dl>
                <dt>SPAN ID</dt>
                <dd>{focus.id.split(":")[1]}</dd>
                <dt>TRACE ROOT</dt>
                <dd>{focus.root.split(":")[1]}</dd>
                <dt>PARENT</dt>
                <dd>{focus.parent.split(":")[1]}</dd>
              </dl>
              <div className="eyebrow attribute-label">ATTRIBUTES</div>
              <div className="attributes">
                {Object.entries(focus.fields).map(([k, v]) => (
                  <div key={k}>
                    <span>{k}</span>
                    <code>{v}</code>
                  </div>
                ))}
              </div>
              {focus.partial && (
                <div className="partial">
                  Start missing from retained history
                </div>
              )}
              <div className="trace-count">
                {spans.filter((p) => p.root === focus.root).length} spans in
                this conversation
              </div>
            </>
          ) : (
            <div className="empty-inspector">
              <div>⌁</div>
              <h3>Follow a hello.</h3>
              <p>
                Select a span or message to inspect its fields and highlight its
                journey between threads.
              </p>
              <div className="flow-example">
                publish <span>→</span> receive <span>→</span> reply
              </div>
            </div>
          )}
        </aside>
      </div>
      {status?.resources && (
        <ResourcePanel
          resources={status.resources}
          spans={s.spans.size}
          events={s.events.length}
        />
      )}
      <section className="panel messages-panel">
        <div className="panel-heading">
          <div>
            <h2>
              Message exchange{" "}
              <span className="count-pill">{messages.length}</span>
            </h2>
            <p>Workers subscribe to “hello” and reply through named inboxes.</p>
          </div>
          <span className="recording">
            <i /> MCAP recording
          </span>
        </div>
        <div className="message-list">
          {messages.slice(0, 50).map((e) => (
            <button
              key={e.id}
              className={`message-row ${selectedRoot === e.root ? "highlight" : ""}`}
              onClick={() => setSelected(e.span)}
            >
              <span className="message-time">{e.time.toFixed(3)}s</span>
              <span className={`phase ${e.fields.phase}`}>
                {e.fields.phase === "publish" ? "↗ PUB" : "↘ SUB"}
              </span>
              <span className="route">
                {e.fields.from ?? e.worker} <b>→</b>{" "}
                {e.fields.to ??
                  (e.fields.topic === "hello"
                    ? "all subscribers"
                    : e.fields.topic)}
              </span>
              <span className="message-text">{e.fields.text}</span>
              <span className="message-id">#{e.fields["message.id"]}</span>
            </button>
          ))}
          {!messages.length && (
            <div className="empty-messages">
              Messages will appear here as workers greet one another.
            </div>
          )}
        </div>
      </section>
      <footer>
        <span>
          <i className="footer-dot" /> Real threads · Crossbeam pub/sub ·
          Protobuf over WebSocket
        </span>
        <span title={status?.recording}>
          {status?.paused
            ? "Publishing paused · queued work drains"
            : `${status?.frequency_hz?.toFixed(1) ?? "…"} new conversations / s`}{" "}
          · 65s retained
        </span>
      </footer>
    </main>
  );
}
createRoot(document.getElementById("root")!).render(<App />);
