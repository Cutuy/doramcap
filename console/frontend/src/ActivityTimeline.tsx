import { useState } from 'react';
import type { Step } from './api';
import { elapsedDuration, shortStep, stepTimes, timelineLane } from './workflow';

const lanes = ['Workflow', 'Health checks', 'Details'];
const left = 118, width = 752;
const colors: Record<string, string> = { STARTED: '#177763', SUCCEEDED: '#4a8bb7', FAILED: '#b13c3c', PANICKED: '#b13c3c', CANCELLED: '#a26a1a' };

export function EventDetails({ step, now, inspectRelated }: { step: Step; now: number; inspectRelated?: (invocation: string) => void }) {
  const times = stepTimes(step);
  return <section className="event-details" aria-labelledby="event-detail-title"><h3 id="event-detail-title">Event details</h3><p className="mono break">{step.step}</p><dl className="detail-grid"><dt>Status</dt><dd>{step.state}</dd><dt>Duration</dt><dd>{elapsedDuration(step, now).toFixed(3)} s</dd><dt>Started at</dt><dd>{times.start.toFixed(3)} s</dd><dt>Invocation</dt><dd>{step.invocation}</dd><dt>Session</dt><dd>{step.session}</dd><dt>Executor</dt><dd>{step.executor}</dd><dt>Parent invocation</dt><dd>{step.parent ? (inspectRelated ? <button className="event-relation" onClick={() => inspectRelated(step.parent!)}>{step.parent}</button> : step.parent) : 'None'}</dd><dt>Dependencies</dt><dd>{step.dependencies?.length ? step.dependencies.map(id => inspectRelated ? <button key={id} className="event-relation" onClick={() => inspectRelated(id)}>{id}</button> : <span key={id}>{id} </span>) : 'None'}</dd><dt>Event sequence</dt><dd>{step.sequence ?? 'Unavailable'}</dd></dl>{step.error && <p className="notice failure-notice" role="status">{step.error}. Inspect private host evidence for the failure diagnostic.</p>}<p className="muted">Input and output payloads are excluded from public telemetry.</p></section>;
}
const identity = (step: Step) => `${step.session}:${step.executor}:${step.invocation}`;

export function ActivityTimeline({ steps, now, live = true }: { steps: Step[]; now: number; live?: boolean }) {
  const [windowSeconds, setWindowSeconds] = useState(15);
  const [paused, setPaused] = useState<{ now: number; steps: Step[] } | null>(null);
  const [selected, setSelected] = useState<Step | null>(null);
  const shown = paused?.steps ?? steps;
  const shownNow = paused?.now ?? now;
  const right = Math.max(windowSeconds, shownNow);
  const start = right - windowSeconds;
  const focus = selected && (shown.find(step => identity(step) === identity(selected)) ?? selected);
  const select = (step: Step) => { setMissingRelated(false); setSelected(step); if (live && !paused) setPaused({ now, steps: [...steps] }); };
  const browse = (step: Step) => { setSelected(step); setPaused({ now: Math.max(windowSeconds, stepTimes(step).end), steps: [...shown] }); };
  const [missingRelated, setMissingRelated] = useState(false);
  const inspectRelated = (invocation: string) => { const target = shown.find(step => step.session === focus?.session && step.executor === focus?.executor && step.invocation === invocation); setMissingRelated(!target); if (target) browse(target); };
  const x = (time: number) => left + (time - start) * width / windowSeconds;
  const visible = shown.flatMap(step => {
    const times = stepTimes(step);
    const end = step.state === 'STARTED' ? Math.max(shownNow, times.end) : times.end;
    if (end < start || times.start > right) return [];
    return [{ step, start: Math.max(start, times.start), end: Math.min(right, end) }];
  });
  return <section className="panel activity-panel" aria-labelledby="activity-title">
    <div className="panel-heading activity-heading"><div><h2 id="activity-title">Activity timeline</h2><p className="muted">{live ? 'Live workflow events, including periodic health checks.' : 'Retained workflow events from this run.'}</p></div><div className="timeline-controls"><label htmlFor="timeline-window">Window</label><select id="timeline-window" value={windowSeconds} onChange={event => setWindowSeconds(Number(event.target.value))}><option value={5}>5 s</option><option value={15}>15 s</option><option value={30}>30 s</option><option value={60}>60 s</option></select><button className="button secondary timeline-toggle" onClick={() => setPaused(paused ? null : { now, steps: [...steps] })}>{paused ? (live ? 'Follow live' : 'Latest events') : (live ? 'Pause' : 'Browse events')}</button></div></div>
    <div className="activity-scroll"><svg className="activity-timeline" viewBox="0 0 900 210" onClick={event => {
      const box = event.currentTarget.getBoundingClientRect();
      const plotX = (event.clientX - box.left) * 900 / box.width;
      const plotY = (event.clientY - box.top) * 210 / box.height;
      if (plotX < left || plotX > left + width || plotY < 34 || plotY > 178) return;
      const lane = Math.floor((plotY - 34) / 48);
      const time = start + (plotX - left) * windowSeconds / width;
      const distance = (item: typeof visible[number]) => Math.max(item.start - time, time - item.end, 0);
      const nearby = visible.filter(item => timelineLane(item.step) === lane).sort((a,b) => distance(a) - distance(b))[0];
      if (nearby) select(nearby.step);
    }} role="group" aria-label={`Workflow activity timeline, ${live ? (paused ? 'paused' : 'following live') : 'retained run'}, showing ${windowSeconds} seconds`}>
      <defs><clipPath id="activity-plot"><rect x={left} y="28" width={width} height="150" /></clipPath></defs>
      {Array.from({ length: 6 }, (_, i) => <g key={i}><line x1={left + i * width / 5} x2={left + i * width / 5} y1="28" y2="178" className="timeline-grid"/><text x={left + i * width / 5} y="17" textAnchor="middle" className="timeline-axis">{(start + i * windowSeconds / 5).toFixed(1)} s</text></g>)}
      {lanes.map((lane, i) => <g key={lane}><rect x={left} y={34 + i * 48} width={width} height="39" rx="5" className="timeline-lane"/><text x="8" y={58 + i * 48} className="timeline-lane-label">{lane}</text></g>)}
      <g clipPath="url(#activity-plot)">{visible.map(({ step, start: from, end }) => {
        const x1 = x(from), spanWidth = Math.max(8, x(end) - x1), markerX = Math.max(left, Math.min(left + width - spanWidth, x1)), y = 43 + timelineLane(step) * 48;
        return <rect className="timeline-event" role="button" tabIndex={0} aria-label={`${shortStep(step)}, ${step.state.toLowerCase()}, invocation ${step.invocation}`} aria-pressed={selected ? identity(selected) === identity(step) : false} onClick={event => { event.stopPropagation(); select(step); }} onKeyDown={event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); select(step); } }} stroke={selected && identity(selected) === identity(step) ? '#20352f' : 'none'} strokeWidth="3" key={identity(step)} x={markerX} y={y} width={spanWidth} height="21" rx="3" fill={colors[step.state] ?? '#72857d'} opacity="0.85"><title>{`${shortStep(step)} · ${step.state.toLowerCase()} · invocation ${step.invocation}`}</title></rect>;
      })}</g>
      {!shown.length && <text x="494" y="111" textAnchor="middle" className="timeline-empty">{live ? 'Waiting for workflow events…' : 'No retained workflow events available.'}</text>}
      <text x={left} y="198" className="timeline-axis">Elapsed since workflow start</text>
    </svg></div>
    <label className="timeline-position" htmlFor="timeline-position">Browse elapsed time · {right.toFixed(1)} s<input id="timeline-position" type="range" min={windowSeconds} max={Math.max(windowSeconds, now)} step="0.1" value={Math.min(right, Math.max(windowSeconds, now))} onChange={event => setPaused({ now: Number(event.target.value), steps: [...steps] })}/></label>
    {missingRelated && <p className="notice">Related invocation is outside the retained activity.</p>}
    {focus ? <><div className="actions"><button className="button secondary" onClick={() => { setSelected(null); setMissingRelated(false); }}>Clear selection</button></div><EventDetails step={focus} now={shownNow} inspectRelated={inspectRelated}/></> : <p className="muted">Select an event to inspect its timing and relationships.</p>}
    <p className="timeline-scroll-hint">Scroll the timeline to see the latest activity.</p><div className="timeline-legend"><span><i className="timeline-key running"/>Running</span><span><i className="timeline-key completed"/>Completed</span><span><i className="timeline-key failed"/>Failed</span><span><i className="timeline-key cancelled"/>Cancelled</span></div>
  </section>;
}
