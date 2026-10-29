import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { TraceStore, TraceSpan, colors, bufferLanes, rootOf, spanExtent } from "./model";
import { zoomTimeline } from "./timeline";
import "./style.css";
const duration = (s: TraceSpan, end: number) => `${Math.max(0, end - s.start) * 1000 < 1 ? (Math.max(0, end - s.start) * 1e6).toFixed(1) + ' µs' : (Math.max(0, end - s.start) * 1000).toFixed(2) + ' ms'}`;
function App() {
    const store = useRef(new TraceStore());
    const [, redraw] = useState(0);
    const [connection, setConnection] = useState("Connecting"), [error, setError] = useState("");
    const [gaps, setGaps] = useState(0), [selected, setSelected] = useState("");
    const [filter, setFilter] = useState(""), [seconds, setSeconds] = useState(5), [frozen, setFrozen] = useState<number | null>(null);
    const svg = useRef<SVGSVGElement>(null);
    const view = useRef({ seconds, frozen });
    view.current = { seconds, frozen };
    useEffect(() => {
        let closed = false, socket: WebSocket | undefined, retry: ReturnType<typeof setTimeout>;
        function connect() {
            if (closed)
                return;
            setConnection("Connecting");
            const url = new URL('../api/activity/stream', document.baseURI);
            url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
            socket = new WebSocket(url);
            socket.binaryType = "arraybuffer";
            socket.onopen = () => { setConnection("Live"); setError(""); };
            socket.onmessage = e => { try {
                if (typeof e.data === 'string') {
                    const m = JSON.parse(e.data);
                    if (m.type === 'gap')
                        setGaps(v => v + Number(m.batches));
                }
                else
                    store.current.ingest(new Uint8Array(e.data));
            }
            catch (e) {
                setError(String(e));
            } };
            socket.onerror = () => setConnection("Reconnecting");
            socket.onclose = () => { if (!closed) {
                setConnection("Reconnecting");
                retry = setTimeout(connect, 1000);
            } };
        }
        fetch(new URL('../api/activity/status', document.baseURI)).then(r => r.json()).then(s => { if (closed)
            return; if (s.enabled)
            connect();
        else
            setConnection("Activity tracing disabled in this build"); }).catch(e => { if (!closed)
            setError(String(e)); });
        const timer = setInterval(() => redraw(v => v + 1), 100);
        return () => { closed = true; clearInterval(timer); clearTimeout(retry); socket?.close(); };
    }, []);
    useEffect(() => { const element = svg.current!; const wheel = (e: WheelEvent) => { const box = element.getBoundingClientRect(); const x = (e.clientX - box.left) / box.width * 1100; if (x < 240)
        return; e.preventDefault(); const v = view.current; const right = Math.max(v.seconds, v.frozen ?? store.current.now()); const next = zoomTimeline(v.seconds, right, (x - 240) / 840, e.deltaY * (e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 500 : 1)); view.current = { seconds: next.seconds, frozen: next.right }; setSeconds(next.seconds); setFrozen(next.right); }; element.addEventListener('wheel', wheel, { passive: false }); return () => element.removeEventListener('wheel', wheel); }, []);
    const s = store.current, right = Math.max(seconds, frozen ?? s.now()), left = right - seconds;
    const lanes = bufferLanes(s, left, right, filter);
    let bottom = 32;
    const layout = lanes.map(lane => {
        const top = bottom;
        bottom += Math.max(55, lane.tracks * 42 + 12);
        return { ...lane, top };
    });
    const height = bottom;
    const instances = lanes.reduce((n, lane) => n + lane.entries.length, 0);
    const children = new Map<string, TraceSpan[]>();
    for (const span of s.spans.values()) {
        if (span.start > right || spanExtent(span, s.spans, right).time < left) continue;
        const list = children.get(span.parent) ?? [];
        list.push(span);
        children.set(span.parent, list);
    }
    const x = (t: number) => 240 + (Math.max(left, Math.min(right, t)) - left) / seconds * 840;
    const picked = s.spans.get(selected), root = picked ? rootOf(picked, s.spans) : undefined;
    const events = s.events.filter(e => e.span === selected);
    const focus = (span: TraceSpan) => { const end = spanExtent(span, s.spans, s.now()).time; const width = Math.max(.001, (end - span.start) * 1.5); setSeconds(Math.min(60, width)); setFrozen(Math.max(width, end + width * .15)); };
    return <main><header><div><h2>Live activity timeline</h2><small>Combining residence · write lifetime · SQE → observed CQE</small></div><span className="status">{connection}</span></header>
 <div className="controls"><button onClick={() => setFrozen(frozen === null ? right : null)}>{frozen === null ? 'Pause view' : 'Follow live'}</button><button aria-label="Earlier" onClick={() => setFrozen(Math.max(seconds, right - seconds / 2))}>←</button><button aria-label="Later" onClick={() => setFrozen(right + seconds / 2)}>→</button><label>Window <select value={seconds} onChange={e => setSeconds(Number(e.target.value))}>{![.001, .01, .1, 1, 5, 15, 60].includes(seconds) && <option value={seconds}>{seconds.toPrecision(3)} s</option>}{[.001, .01, .1, 1, 5, 15, 60].map(n => <option key={n} value={n}>{n < 1 ? n * 1000 + ' ms' : n + ' s'}</option>)}</select></label><input aria-label="Filter activity" value={filter} onChange={e => setFilter(e.target.value)} placeholder="Path, writer, write, file or region"/>{picked && <button onClick={() => focus(picked)}>Zoom to selection</button>}</div>
 <div className="legend">{Object.entries(colors).map(([path, color]) => <span key={path}><i style={{ background: color }}/>{path}</span>)}</div>
 <div className="loss" role="status">{s.received} records · sequence gaps {s.gaps} · transport gaps {gaps} · producer record loss {s.producerLoss} · field loss {s.fieldLoss} · evicted spans {s.evicted} · {lanes.length} buffer lanes · {instances} instances</div>{error && <p role="alert">{error}</p>}
 <div className="plot"><svg ref={svg} viewBox={`0 0 1100 ${height}`} width="100%" role="img" aria-label="Live storage write and I/O spans">
 {Array.from({ length: 6 }, (_, i) => { const px = 240 + i * 168; return <g key={i}><line x1={px} x2={px} y1={25} y2={height} stroke="#303b34"/><text x={px} y={18} fontSize={11} fill="#98a79c">{(left + i * seconds / 5).toFixed(seconds < .1 ? 4 : 2)} s</text></g>; })}
 {layout.map(lane => {
            const color = colors[lane.name] ?? '#99a7bb';
            const bar = (v: TraceSpan, yy: number, h: number, fill: string) => {
                const extent = spanExtent(v, s.spans, right);
                const start = x(v.start), end = x(extent.time);
                return <g key={v.id} role="button" tabIndex={0} aria-label={`${v.name} ${v.fields.path ?? ''} ${v.fields.write ?? v.fields.operation ?? ''}`} onClick={() => setSelected(v.id)} onKeyDown={e => {
                    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setSelected(v.id); }
                }}><rect x={start} y={yy} width={Math.max(3, end - start)} height={h} rx={3} fill={fill} opacity={v.partial || extent.uncertain ? .55 : 1} stroke={selected === v.id ? '#fff' : fill} strokeWidth={selected === v.id ? 2 : 1} strokeDasharray={v.end === undefined ? '4 2' : undefined}/><title>{v.name} · {duration(v, extent.time)} · {extent.status} · {JSON.stringify(v.fields)}{v.partial ? ' · incomplete history' : ''}</title></g>;
            };
            return <g key={lane.name} aria-label={`${lane.name} buffer lane`}>
                <line x1={0} x2={1100} y1={lane.top} y2={lane.top} stroke="#303b34"/>
                <text x={8} y={lane.top + 20} fill={color} fontSize={13}>{lane.name}</text>
                <text x={8} y={lane.top + 37} fill="#98a79c" fontSize={10}>{lane.entries.length} instances</text>
                {!lane.entries.length && <text x={250} y={lane.top + 25} fill="#66796b" fontSize={11}>No activity in this window</text>}
                {lane.entries.map(({span, track}) => {
                    const y = lane.top + 8 + track * 42;
                    return <g key={span.id}>{bar(span, y, 17, color)}{(children.get(span.id) ?? []).map(c => bar(c, y + 22, 10, '#dfe8e2'))}</g>;
                })}
            </g>;
        })}</svg></div>
 <section className="details"><strong>{picked ? `${picked.name} · ${duration(picked, spanExtent(picked, s.spans, right).time)}${picked.partial || spanExtent(picked, s.spans, right).uncertain ? " (incomplete interval)" : ""}` : 'Select a write or an I/O operation'}</strong>{picked && <><span> {spanExtent(picked, s.spans, right).status}{picked.partial ? ' · incomplete history' : ''}</span><div className="fields">{Object.entries(picked.fields).map(([k, v]) => <span key={k}>{k}: <b>{v}</b></span>)}</div>{root && root.id !== picked.id && <button onClick={() => setSelected(root.id)}>Parent write #{root.fields.write}</button>}{picked.links.map(id => <button key={id} disabled={!s.spans.has(id)} onClick={() => setSelected(id)}>Linked combining batch{s.spans.has(id) ? '' : ' (evicted)'}</button>)}{events.map(e => <div key={e.id} className="event">{e.time.toFixed(6)} s · {Object.entries(e.fields).map(([k, v]) => `${k}=${v}`).join(' · ')}</div>)}</>}</section>
 </main>;
}
createRoot(document.getElementById('root')!).render(<App />);
