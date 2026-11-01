import { StepState } from './api';
import type { Step } from './api';
import { StatusLabel } from './ui/StatusLabel';
import { elapsedDuration, summarySteps, stepName } from './workflow';

const labels: Partial<Record<Step['state'], string>> = { [StepState.STARTED]: 'Running', [StepState.SUCCEEDED]: 'Completed', [StepState.FAILED]: 'Failed', [StepState.CANCELLED]: 'Cancelled', [StepState.PANICKED]: 'Panicked' };
export function RunSteps({ steps, now = 0, emptyMessage = 'Waiting for workflow steps.' }: { steps: Step[]; now?: number; emptyMessage?: string }) {
  const stages = summarySteps(steps);
  return <div className="table-wrap"><table><thead><tr><th>Step</th><th>Status</th><th className="duration">Elapsed</th></tr></thead><tbody>{stages.map(step => {
    const label = labels[step.state] ?? 'Unknown';
    const tone = step.state === StepState.STARTED ? 'active' : step.state === StepState.SUCCEEDED ? 'success' : [StepState.FAILED, StepState.PANICKED].includes(step.state) ? 'failure' : 'neutral';
    return <tr key={`${step.session}:${step.executor}:${String(step.invocation)}`}><td><span title={step.step}>{stepName(step)}</span><span className="sr-only"> {step.step}, invocation {String(step.invocation)}</span></td><td><StatusLabel label={label} tone={tone}/></td><td className="duration">{elapsedDuration(step, now).toFixed(1)} s</td></tr>;
  })}</tbody></table>{stages.length === 0 && <p className="muted">{emptyMessage}</p>}</div>;
}
