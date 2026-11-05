import type { ReactNode } from 'react';
import { useConsole } from '../context';
import { historyHash, runHash } from '../model';
export type PagePhase = 'Project' | 'Review' | 'Run' | 'Result';
export type ConsoleView = { label: string; href: string; current: boolean };
/** What a frame arranges: top-level view links, the run phase rail and the current screen. */
export type FrameParts = { views: ConsoleView[]; rail: ReactNode; screen: ReactNode };
/**
 * Arrange the parts of Console on the page.
 * Called during render, so it may be an inline closure without remounting the screens.
 */
export type Frame = (parts: FrameParts) => ReactNode;

/** The themed Console surface for a host's frame. Hosts render `rail` and `screen` inside it. */
export function ConsoleWindow({ rail, children }: { rail: ReactNode; children: ReactNode }) {
  return <div className="console-root"><div className="page">{rail && <div className="console-nav">{rail}</div>}{children}</div></div>;
}

const standaloneFrame: Frame = ({ views, rail, screen }) => <div className="console-root"><header className="topbar"><strong>doramcap</strong><span className="divider"/><span>Console</span><nav className="topnav" aria-label="Console views">{views.map(view => <a key={view.label} href={view.href} aria-current={view.current ? 'page' : undefined}>{view.label}</a>)}</nav>{rail}<span className="host">Single host</span></header><main className="page">{screen}</main></div>;

export function PageFrame({ phase, children, onNavigate, history = false, frame = standaloneFrame }: { phase: PagePhase; children: ReactNode; onNavigate?: (phase: PagePhase) => void; history?: boolean; frame?: Frame }) {
  const { basePath } = useConsole();
  const phases: PagePhase[] = ['Project', 'Review', 'Run', 'Result'];
  const views: ConsoleView[] = [{ label: 'Run', href: runHash(basePath), current: !history }, { label: 'History', href: historyHash('', basePath), current: history }];
  const rail = !history && <nav className="phase-rail" aria-label="Run phases">{phases.map(item => onNavigate && (item === 'Run' || item === 'Result') ? <button key={item} className={item === phase ? 'phase phase-link current' : 'phase phase-link'} aria-current={item === phase ? 'step' : undefined} onClick={() => onNavigate(item)}>{item}</button> : <span key={item} className={item === phase ? 'phase current' : 'phase'} aria-current={item === phase ? 'step' : undefined}>{item}</span>)}</nav>;
  return frame({ views, rail, screen: children });
}
