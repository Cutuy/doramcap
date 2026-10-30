import type { ReactNode } from 'react';
export type PagePhase = 'Project' | 'Review' | 'Run' | 'Result';
export function PageFrame({ phase, children, onNavigate }: { phase: PagePhase; children: ReactNode; onNavigate?: (phase: PagePhase) => void }) {
  const phases: PagePhase[] = ['Project', 'Review', 'Run', 'Result'];
  return <><header className="topbar"><strong>doramcap</strong><span className="divider"/><span>Console</span><span className="host">Single host</span></header><main className="page"><nav className="phase-rail" aria-label="Run phases">{phases.map((item, index) => onNavigate && (item === 'Run' || item === 'Result') ? <button key={item} className={item === phase ? 'phase phase-link current' : 'phase phase-link'} aria-current={item === phase ? 'step' : undefined} onClick={() => onNavigate(item)}><small>0{index + 1}</small>{item}</button> : <span key={item} className={item === phase ? 'phase current' : 'phase'} aria-current={item === phase ? 'step' : undefined}><small>0{index + 1}</small>{item}</span>)}</nav>{children}</main></>;
}
