import { artifactFilename } from './generated/transport';
import { useConsole } from './context';
import { compareHash } from './model';
import { Phase } from './api';
import type { ReactNode } from 'react';
import type { HistoryArtifact, Snapshot } from './api';
import { MachineChanges } from './Comparison';
import { bytesLabel } from './RunMetrics';
import { ActionButton } from './ui/ActionButton';
import { OutcomeSummary } from './ui/OutcomeSummary';

/** `past` shows a stored run: its stored title, action and on-disk artifact facts. */
export type PastRun = { title: string; action: string; disabled: boolean; artifacts: HistoryArtifact[]; aside: ReactNode; error?: string | null };
export function Result({ snapshot, newRun, viewRun, past }: { snapshot: Snapshot; newRun: () => void; viewRun?: () => void; past?: PastRun }) {
  const { client, basePath } = useConsole();
  const outputs = snapshot.recordings ?? [];
  const blocked = snapshot.phase === Phase.BLOCKED;
  const saved = snapshot.outcome?.recordingFinalized === true;
  const artifact = (id: string) => past?.artifacts.find(item => item.id === id);
  const ready = (id: string) => past ? artifact(id)?.downloadable === true : saved;
  const recordingUrl = (id: string) => ready(id) ? client.artifactUrl(snapshot.runId, id) : undefined;
  const reportUrl = snapshot.reportAvailable ? client.artifactUrl(snapshot.runId, 'report') : undefined;
  const error = past ? past.error : snapshot.outcome?.error;
  const killed = saved ? (snapshot.outcome?.serviceExits ?? []).filter(exit => exit.killed) : [];
  const size = (id: string) => artifact(id)?.sizeBytes == null ? '' : ` · ${bytesLabel(artifact(id)!.sizeBytes)}`;
  return <>
    <div className="heading"><div><h1>{past ? past.title : blocked ? 'Cleanup unverified' : snapshot.outcome?.execution === 'Failed' ? 'Run failed' : snapshot.outcome?.execution === 'Cancelled' ? 'Run cancelled' : 'Run complete'}</h1><p>Run <span className="mono">{snapshot.runId || 'Unknown'}</span></p></div><div className="run-view-actions">{viewRun && <ActionButton variant="secondary" onClick={viewRun}>View run activity</ActionButton>}{!past && snapshot.runId && <a className="button secondary" href={compareHash([snapshot.runId], basePath)}>Compare</a>}<ActionButton variant={past ? 'primary' : 'secondary'} disabled={past?.disabled} onClick={newRun}>{past ? past.action : 'New run'}</ActionButton></div></div>
    {error && <p className="notice failure-notice run-error" role="alert">{error}</p>}
    {killed.length > 0 && <p className="notice" role="status">Killed: {killed.map(exit => `${exit.service} (exit ${exit.exitCode}${exit.oomKilled ? ', out of memory' : ''})`).join(', ')}. Final messages may be missing from this recording.</p>}
    <div className="columns"><div><OutcomeSummary outcome={snapshot.outcome ?? null} recordingEnabled={outputs.length > 0}/><section className="panel"><h2>Evidence</h2><div className="actions">
      {outputs.length ? outputs.map(output => recordingUrl(output.id) ? <a className="button primary" key={output.id} href={recordingUrl(output.id)} download={artifactFilename(output.id)}>Download {output.id} recording{size(output.id)}</a> : <span className="unavailable" key={output.id}>{output.id} recording unavailable{size(output.id) && `${size(output.id)} left on host`}</span>) : <span className="unavailable">Recording disabled</span>}
      {reportUrl ? <a className="button secondary" href={reportUrl} download={artifactFilename('report')}>Download report</a> : <span className="unavailable">Report unavailable</span>}
    </div></section></div>{past ? <aside>{past.aside}</aside> : blocked && <aside><section className="panel attention-panel"><h2>Reset required</h2><p>The next run will ask the repository controller to reset the experiment before starting. If reset fails, the run stays blocked; inspect the private report on the host.</p></section></aside>}</div>
    {!past && snapshot.machineChanges.length > 0 && <MachineChanges rows={snapshot.machineChanges}/>}
  </>;
}
