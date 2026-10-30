import { useEffect, useRef, useState } from 'react';
import type { Snapshot } from './api';
import { ActivityTimeline } from './ActivityTimeline';
import { RunSteps } from './RunSteps';
import { StatusLabel } from './ui/StatusLabel';
import { stepTimes } from './workflow';

export function RunActivity({ snapshot, stopping, disconnected, historical = false }: { snapshot: Snapshot | null; stopping: boolean; disconnected: boolean; historical?: boolean }) {
  const [clock, setClock] = useState(() => Date.now());
  const anchor = useRef({ runId: '', sequence: '', elapsed: 0, wall: Date.now() });
  useEffect(() => { if (historical) return; const timer = setInterval(() => setClock(Date.now()), 250); return () => clearInterval(timer); }, [historical]);
  const steps = snapshot?.steps ?? [];
  const runId = snapshot?.runId ?? '';
  const sequence = snapshot?.sequence ?? '';
  if (anchor.current.runId !== runId || anchor.current.sequence !== sequence) {
    anchor.current = { runId, sequence, elapsed: Math.max(0, ...steps.map(step => stepTimes(step).end)), wall: Date.now() };
  }
  const now = anchor.current.elapsed + (historical || disconnected || !steps.length ? 0 : Math.max(0, clock - anchor.current.wall) / 1000);
  return <><section className="panel"><div className="panel-heading"><h2>Workflow steps</h2><StatusLabel label={historical ? (snapshot?.phase === 'FINISHED' ? (snapshot.outcome?.execution === 'Cancelled' ? 'Cancelled run' : 'Completed run') : 'Interrupted run') : stopping ? 'Stopping' : 'Running'} tone={historical ? 'neutral' : 'active'}/></div>{snapshot?.progressIncomplete && <p className="notice">Progress incomplete</p>}{snapshot && snapshot.truncatedSteps !== '0' && <p className="notice">Older activity hidden</p>}<RunSteps steps={steps} now={now} emptyMessage={historical ? 'No workflow steps available.' : undefined}/></section><ActivityTimeline key={runId} steps={steps} now={now} live={!historical}/></>;
}
