import { useState } from 'react';
import type { Step } from './api';
import { shortStep, stepTimes, timelineLane } from './workflow';

const lanes = ['Workflow', 'Health checks', 'Details'];
const left = 118, width = 752;
const colors: Record<string, string> = { STARTED: '#177763', SUCCEEDED: '#4a8bb7', FAILED: '#b13c3c', PANICKED: '#b13c3c', CANCELLED: '#a26a1a' };

export function ActivityTimeline({ steps, now }: { steps: Step[]; now: number }) {
  const [windowSeconds, setWindowSeconds] = useState(15);
  const [paused, setPaused] = useState<{ now: number; steps: Step[] } | null>(null);
  const shown = paused?.steps ?? steps;
  const shownNow = paused?.now ?? now;
  const right = Math.max(windowSeconds, shownNow);
  const start = right - windowSeconds;
  const x = (time: number) => left + (time - start) * width / windowSeconds;
  const visible = shown.flatMap(step => {
    const times = stepTimes(step);
    const end = step.state === 'STARTED' ? Math.max(shownNow, times.end) : times.end;
    if (end < start || times.start > right) return [];
    return [{ step, start: Math.max(start, times.start), end: Math.min(right, end) }];
  });
  return <section className="panel activity-panel" aria-labelledby="activity-title">
    <div className="panel-heading activity-heading"><div><h2 id="activity-title">Activity timeline</h2><p className="muted">Live workflow events, including periodic health checks.</p></div><div className="timeline-controls"><label htmlFor="timeline-window">Window</label><select id="timeline-window" value={windowSeconds} onChange={event => setWindowSeconds(Number(event.target.value))}><option value={5}>5 s</option><option value={15}>15 s</option><option value={30}>30 s</option><option value={60}>60 s</option></select><button className="button secondary timeline-toggle" onClick={() => setPaused(paused ? null : { now, steps: [...steps] })}>{paused ? 'Follow live' : 'Pause'}</button></div></div>
    <div className="activity-scroll"><svg className="activity-timeline" viewBox="0 0 900 210" role="img" aria-label={`Workflow activity timeline, ${paused ? 'paused' : 'following live'}, showing ${windowSeconds} seconds`}>
      <defs><clipPath id="activity-plot"><rect x={left} y="28" width={width} height="150" /></clipPath></defs>
      {Array.from({ length: 6 }, (_, i) => <g key={i}><line x1={left + i * width / 5} x2={left + i * width / 5} y1="28" y2="178" className="timeline-grid"/><text x={left + i * width / 5} y="17" textAnchor="middle" className="timeline-axis">{(start + i * windowSeconds / 5).toFixed(1)} s</text></g>)}
      {lanes.map((lane, i) => <g key={lane}><rect x={left} y={34 + i * 48} width={width} height="39" rx="5" className="timeline-lane"/><text x="8" y={58 + i * 48} className="timeline-lane-label">{lane}</text></g>)}
      <g clipPath="url(#activity-plot)">{visible.map(({ step, start: from, end }) => {
        const x1 = x(from), spanWidth = Math.max(3, x(end) - x1), y = 43 + timelineLane(step) * 48;
        return <rect key={`${step.session}:${step.executor}:${step.invocation}`} x={x1} y={y} width={spanWidth} height="21" rx="3" fill={colors[step.state] ?? '#72857d'} opacity="0.85"><title>{`${shortStep(step)} · ${step.state.toLowerCase()} · invocation ${step.invocation}`}</title></rect>;
      })}</g>
      {!shown.length && <text x="494" y="111" textAnchor="middle" className="timeline-empty">Waiting for workflow events…</text>}
      <text x={left} y="198" className="timeline-axis">Elapsed since workflow start</text>
    </svg></div>
    <p className="timeline-scroll-hint">Scroll the timeline to see the latest activity.</p><div className="timeline-legend"><span><i className="timeline-key running"/>Running</span><span><i className="timeline-key completed"/>Completed</span><span><i className="timeline-key failed"/>Failed</span><span><i className="timeline-key cancelled"/>Cancelled</span></div>
  </section>;
}
