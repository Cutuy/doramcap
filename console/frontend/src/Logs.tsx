import { doramcap } from './generated/console';
import type { Snapshot } from './api';
import { bytesLabel } from './RunMetrics';

const { LogState } = doramcap.journal.v1;
const labels: Record<number, string> = {
  [LogState.LOG_STATE_PENDING]: 'Pending', [LogState.LOG_STATE_EXPORTED]: 'Exported',
  [LogState.LOG_STATE_SKIPPED]: 'Skipped', [LogState.LOG_STATE_FAILED]: 'Failed',
};

/** Run-local journal evidence. Numbered files can attach to the shared run-file route. */
export function Logs({ snapshot }: { snapshot: Snapshot }) {
  return <section className="panel logs"><h2>Logs</h2>
    <p>Exported files contain the entries still available in the host journal.</p>
    {snapshot.logsError ? <p role="alert">{snapshot.logsError}</p> : snapshot.logs?.programs.length ?
      <ul className="log-list">{snapshot.logs.programs.map((program, number) => <li key={number}>
        <div className="log-program mono">{program.programId}</div>
        <span>{labels[program.state ?? 0] ?? 'Unknown'}</span>
        <span>{program.state === LogState.LOG_STATE_SKIPPED || program.state === LogState.LOG_STATE_PENDING ? '—' : bytesLabel(program.sizeBytes)}</span>
        {program.reason && <p className="log-reason">{program.reason}</p>}
      </li>)}</ul> : <p className="unavailable">Logs unavailable for this run.</p>}
  </section>;
}
