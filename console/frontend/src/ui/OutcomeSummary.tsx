import type { Outcome } from '../api';
import { StatusLabel } from './StatusLabel';
import { outcomeLabels } from '../model';
export function OutcomeSummary({ outcome }: { outcome: Outcome | null }) {
  const { execution, recording, cleanup } = outcomeLabels(outcome);
  return <section className="panel"><h2>Run outcomes</h2><div className="outcomes"><div><span>Execution</span><StatusLabel label={execution} tone={execution === 'Failed' ? 'failure' : execution === 'Unknown' ? 'neutral' : execution === 'Cancelled' ? 'attention' : 'success'}/></div><div><span>Recording</span><StatusLabel label={recording} tone={recording === 'Saved' ? 'success' : recording === 'Unavailable' ? 'attention' : 'neutral'}/></div><div><span>Cleanup</span><StatusLabel label={cleanup} tone={cleanup === 'Verified' ? 'success' : cleanup === 'Unverified' ? 'failure' : 'neutral'}/></div></div></section>;
}
