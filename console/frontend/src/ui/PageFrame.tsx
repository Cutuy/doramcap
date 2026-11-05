import type { ReactNode } from 'react';
import { useConsole } from '../context';
import { historyHash, runHash } from '../model';
export type PagePhase = 'Project' | 'Review' | 'Run' | 'Result';
const PHASES: PagePhase[] = ['Project', 'Review', 'Run', 'Result'];
export type ConsoleView = { label: string; href: string; current: boolean };
/** One step of a run. `onSelect` is set when the visitor can move to this step. */
export type ConsolePhase = { label: PagePhase; current: boolean; onSelect?: () => void };
/** What a frame arranges: view links, run phases (empty on History) and the current screen. */
export type FrameParts = { views: ConsoleView[]; phases: ConsolePhase[]; screen: ReactNode };
/**
 * Arrange the parts of Console on the page.
 * Called during render, so it may be an inline closure without remounting the screens.
 */
export type Frame = (parts: FrameParts) => ReactNode;

/** The themed Console surface for a host's frame. Hosts render `screen` inside it. */
export function ConsoleWindow({ children }: { children: ReactNode }) {
  return <div className="console-root"><div className="page">{children}</div></div>;
}

const standaloneRail = (phases: ConsolePhase[]) => phases.length > 0 && <nav className="phase-rail" aria-label="Run phases">{phases.map(({ label, current, onSelect }) => onSelect ? <button key={label} className={current ? 'phase phase-link current' : 'phase phase-link'} aria-current={current ? 'step' : undefined} onClick={onSelect}>{label}</button> : <span key={label} className={current ? 'phase current' : 'phase'} aria-current={current ? 'step' : undefined}>{label}</span>)}</nav>;

const standaloneFrame: Frame = ({ views, phases, screen }) => <div className="console-root"><header className="topbar"><strong>doramcap</strong><span className="divider"/><span>Console</span><nav className="topnav" aria-label="Console views">{views.map(view => <a key={view.label} href={view.href} aria-current={view.current ? 'page' : undefined}>{view.label}</a>)}</nav>{standaloneRail(phases)}<span className="host">Single host</span></header><main className="page">{screen}</main></div>;

export function PageFrame({ phase, children, onNavigate, history = false, frame = standaloneFrame }: { phase: PagePhase; children: ReactNode; onNavigate?: (phase: PagePhase) => void; history?: boolean; frame?: Frame }) {
  const { basePath } = useConsole();
  const views: ConsoleView[] = [{ label: 'Run', href: runHash(basePath), current: !history }, { label: 'History', href: historyHash('', basePath), current: history }];
  const phases: ConsolePhase[] = history ? [] : PHASES.map(label => ({ label, current: label === phase, onSelect: onNavigate && (label === 'Run' || label === 'Result') ? () => onNavigate(label) : undefined }));
  return frame({ views, phases, screen: children });
}
