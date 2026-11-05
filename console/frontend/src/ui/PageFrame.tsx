import type { ReactNode } from 'react';
import { useConsole } from '../context';
import { historyHash, runHash } from '../model';
export type PagePhase = 'Project' | 'Review' | 'Run' | 'Result';
export function PageFrame({ phase, children, onNavigate, history = false }: { phase: PagePhase; children: ReactNode; onNavigate?: (phase: PagePhase) => void; history?: boolean }) {
  const { basePath } = useConsole();
  const phases: PagePhase[] = ['Project', 'Review', 'Run', 'Result'];
  const views = <nav className="topnav" aria-label="Console views"><a href={runHash(basePath)} aria-current={history ? undefined : 'page'}>Run</a><a href={historyHash('', basePath)} aria-current={history ? 'page' : undefined}>History</a></nav>;
  const rail = !history && <nav className="phase-rail" aria-label="Run phases">{phases.map(item => onNavigate && (item === 'Run' || item === 'Result') ? <button key={item} className={item === phase ? 'phase phase-link current' : 'phase phase-link'} aria-current={item === phase ? 'step' : undefined} onClick={() => onNavigate(item)}>{item}</button> : <span key={item} className={item === phase ? 'phase current' : 'phase'} aria-current={item === phase ? 'step' : undefined}>{item}</span>)}</nav>;
  // An embedded host owns the page header and main landmark.
  if (basePath) return <div className="page"><div className="console-nav">{views}{rail}</div>{children}</div>;
  return <><header className="topbar"><strong>doramcap</strong><span className="divider"/><span>Console</span>{views}{rail}<span className="host">Single host</span></header><main className="page">{children}</main></>;
}
