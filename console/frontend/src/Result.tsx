import type { Snapshot } from './api';
import { ActionButton } from './ui/ActionButton';
import { OutcomeSummary } from './ui/OutcomeSummary';

export function Result({ snapshot, newRun, viewRun }: { snapshot: Snapshot; newRun: () => void; viewRun?: () => void }) {
  const outputs = snapshot.recordings ?? [];
  const blocked = snapshot.phase === 'BLOCKED';
  const saved = snapshot.outcome?.recordingFinalized === true;
  return <>
    <div className="heading"><div><h1>{blocked ? 'Cleanup unverified' : snapshot.outcome?.execution === 'Failed' ? 'Run failed' : snapshot.outcome?.execution === 'Cancelled' ? 'Run cancelled' : 'Run complete'}</h1><p>Run <span className="mono">{snapshot.runId || 'Unknown'}</span></p></div><div className="run-view-actions">{viewRun && <ActionButton variant="secondary" onClick={viewRun}>View run activity</ActionButton>}<ActionButton variant="secondary" onClick={newRun}>New run</ActionButton></div></div>
    <div className="columns"><div><OutcomeSummary outcome={snapshot.outcome} recordingEnabled={outputs.length > 0}/><section className="panel"><h2>Evidence</h2><div className="actions">
      {outputs.length ? outputs.map(output => saved ? <a className="button primary" key={output.id} href={`/api/artifacts/${encodeURIComponent(snapshot.runId)}/${encodeURIComponent(output.id)}`}>Download {output.id} recording</a> : <span className="unavailable" key={output.id}>{output.id} recording unavailable</span>) : <span className="unavailable">Recording disabled</span>}
      {snapshot.reportAvailable ? <a className="button secondary" href={`/api/artifacts/${encodeURIComponent(snapshot.runId)}/report`}>Download report</a> : <span className="unavailable">Report unavailable</span>}
    </div></section></div>{blocked && <aside><section className="panel attention-panel"><h2>Reset required</h2><p>The next run will ask the repository controller to reset the experiment before starting. If reset fails, the run stays blocked; inspect the private report on the host.</p></section></aside>}</div>
  </>;
}
