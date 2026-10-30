import type { Step } from './api';

/** Older captures without descriptors retain a diagnostic display fallback. */
export const shortStep = (step: Step) => step.descriptor?.displayName || step.step.split('::').at(-1) || step.step;
export const stepName = shortStep;
export const stepKey = (step: Step) => step.descriptor?.id || step.step;
const summaryKey = (step: Step) => JSON.stringify([step.session, step.executor, stepKey(step)]);
const sameSummaryStep = (left: Step, right: Step) => left.parent == null && right.parent == null && summaryKey(left) === summaryKey(right);

/** Console summary shows the latest invocation of each top-level step. */
export const summarySteps = (steps: Step[]): Step[] => [...new Map(steps.filter(step => step.parent == null).map(step => [summaryKey(step), step])).values()];

/** Prefer completed children/repeats; fall back to the oldest completed row. */
export function evictionIndex(steps: Step[], incoming: Step): number {
  const preferred = steps.findIndex((row, index) => row.state !== 'STARTED'
    && (row.parent != null || sameSummaryStep(row, incoming) || steps.slice(index + 1).some(later => sameSummaryStep(row, later))));
  return preferred >= 0 ? preferred : steps.findIndex(row => row.state !== 'STARTED');
}

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
