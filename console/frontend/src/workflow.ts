import type { Step } from './api';

const lifecycleNames: Record<string, string> = {
  PrepareRun: 'Prepare project',
  PrepareOverlays: 'Prepare filesystem',
  LaunchProjects: 'Start project',
  QuiesceProjects: 'Stop project',
  FinalizeRun: 'Finalize recording',
  RemoveProjects: 'Remove resources',
  WriteRunReport: 'Write run report',
};

export const shortStep = (step: Step) => step.step.split('::').at(-1) ?? step.step;
export const stepName = (step: Step) => lifecycleNames[shortStep(step)] ?? shortStep(step);
export function isLifecycleStep(step: Step): boolean {
  return step.parent == null && Object.hasOwn(lifecycleNames, shortStep(step));
}
export const isHealthCheck = (step: Step) => ['ObserveProjects', 'InspectCompose'].includes(shortStep(step));

/** Convert elapsed and duration together so the 64-bit subtraction stays exact. */
export function stepTimes(step: Step): { start: number; end: number } {
  try {
    const elapsed = BigInt(step.elapsedNs ?? '0');
    const duration = BigInt(step.durationNs ?? '0');
    const start = elapsed > duration ? elapsed - duration : 0n;
    return { start: Number(start) / 1e9, end: Number(elapsed) / 1e9 };
  } catch {
    return { start: 0, end: 0 };
  }
}
export function elapsedDuration(step: Step, now: number): number {
  const { start, end } = stepTimes(step);
  return Math.max(0, (step.state === 'STARTED' ? Math.max(now, end) : end) - start);
}
export function timelineLane(step: Step): number {
  return isHealthCheck(step) ? 1 : isLifecycleStep(step) ? 0 : 2;
}
