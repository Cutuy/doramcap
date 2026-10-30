import type { Step } from './api';
import { StatusLabel } from './ui/StatusLabel';
import { elapsedDuration, isLifecycleStep, stepName } from './workflow';

const labels: Record<string, string> = { STARTED: 'Running', SUCCEEDED: 'Completed', FAILED: 'Failed', CANCELLED: 'Cancelled', PANICKED: 'Panicked' };
export function RunSteps({ steps, now = 0 }: { steps: Step[]; now?: number }) {
  const stages = steps.filter(isLifecycleStep);
  return <div className="table-wrap"><table><thead><tr><th>Step</th><th>Status</th><th className="duration">Elapsed</th></tr></thead><tbody>{stages.map(step => {
    const label = labels[step.state] ?? 'Unknown';
    const tone = step.state === 'STARTED' ? 'active' : step.state === 'SUCCEEDED' ? 'success' : ['FAILED', 'PANICKED'].includes(step.state) ? 'failure' : 'neutral';
    return <tr key={`${step.session}:${step.executor}:${step.invocation}`}><td><span title={step.step}>{stepName(step)}</span><span className="sr-only"> {step.step}, invocation {step.invocation}</span></td><td><StatusLabel label={label} tone={tone}/></td><td className="duration">{elapsedDuration(step, now).toFixed(1)} s</td></tr>;
  })}</tbody></table>{stages.length === 0 && <p className="muted">Waiting for workflow steps.</p>}</div>;
}
