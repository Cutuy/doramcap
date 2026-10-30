import type { Step } from './api';
import { stepTimes } from './workflow';

export const invocationKey = (step: Step) => JSON.stringify([step.session, step.executor, step.invocation]);
export interface Flow { source: Step; target: Step; kind: 'parent' | 'dependency'; from: number; to: number }

/** Group by full Rust type name; resolve only explicit, executor-local relationships. */
export function timelineModel(steps: Step[]) {
  const rows = [...new Set(steps.map(step => step.step))];
  const rowByName = new Map(rows.map((name, index) => [name, index]));
  const byInvocation = new Map(steps.map(step => [invocationKey(step), step]));
  const flows: Flow[] = [];
  for (const target of steps) {
    const add = (id: string, kind: Flow['kind']) => {
      const source = byInvocation.get(invocationKey({ ...target, invocation: id }));
      if (!source || source === target) return;
      const times = stepTimes(source);
      flows.push({ source, target, kind, from: kind === 'parent' ? times.start : times.end, to: stepTimes(target).start });
    };
    if (target.parent) add(target.parent, 'parent');
    for (const id of new Set(target.dependencies ?? [])) add(id, 'dependency');
  }
  return { rows, rowByName, flows };
}

/** Route arrowheads to the top of a bar, including relationships within one row. */
export function flowPath(fromX: number, fromY: number, toX: number, toY: number): string {
  if (fromY === toY) return `M${fromX},${fromY} C${fromX},${fromY - 25} ${toX},${toY - 25} ${toX},${toY}`;
  const sourceY = fromY < toY ? fromY + 21 : fromY;
  const targetY = fromY < toY ? toY : toY + 21;
  const middle = (sourceY + targetY) / 2;
  return `M${fromX},${sourceY} C${fromX},${middle} ${toX},${middle} ${toX},${targetY}`;
}
