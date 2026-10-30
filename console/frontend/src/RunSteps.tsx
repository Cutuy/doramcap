import type { Step } from './api';
import { StatusLabel } from './ui/StatusLabel';
const friendly: Record<string, string> = { PrepareRun: 'Prepare project', PrepareSystemApps: 'Prepare CPU recording', ResolveCompose: 'Resolve project', PrepareOverlays: 'Prepare filesystem', LaunchProjects: 'Start application', LaunchDoraFlows: 'Start supporting flows', ObserveProjects: 'Observe application', QuiesceProjects: 'Stop application', FinalizeRun: 'Finalize recording', RemoveProjects: 'Remove resources', WriteRunReport: 'Write run report' };
const labels: Record<string, string> = { STARTED: 'Running', SUCCEEDED: 'Completed', FAILED: 'Failed', CANCELLED: 'Cancelled', PANICKED: 'Panicked' };
export function RunSteps({ steps }: { steps: Step[] }) {
  return <div className="table-wrap"><table><thead><tr><th>Step</th><th>Status</th><th className="duration">Duration</th></tr></thead><tbody>{steps.map(step => {
    const short = step.step.split('::').at(-1) ?? step.step;
    const name = friendly[short] ?? step.step;
    const label = labels[step.state] ?? 'Unknown';
    const tone = step.state === 'STARTED' ? 'active' : step.state === 'SUCCEEDED' ? 'success' : ['FAILED', 'PANICKED'].includes(step.state) ? 'failure' : 'neutral';
    const duration = step.state === 'STARTED' ? '—' : `${(Number(step.durationNs) / 1e9).toFixed(1)} s`;
    return <tr key={`${step.session}:${step.executor}:${step.invocation}`}><td><span title={step.step}>{name}</span><span className="sr-only"> {step.step}, invocation {step.invocation}</span></td><td><StatusLabel label={label} tone={tone}/></td><td className="duration">{duration}</td></tr>;
  })}</tbody></table>{steps.length === 0 && <p className="muted">Waiting for workflow steps.</p>}</div>;
}
