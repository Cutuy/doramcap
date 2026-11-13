import { doramcap } from './generated/console';
import type { Snapshot } from './api';
import { bytesLabel } from './RunMetrics';
import { useConsole } from './context';
import { RUN_FILE_ARTIFACT } from './generated/transport';

const { LogState } = doramcap.journal.v1;
const labels: Record<number, string> = {
  [LogState.LOG_STATE_PENDING]: 'Pending', [LogState.LOG_STATE_EXPORTED]: 'Exported',
  [LogState.LOG_STATE_SKIPPED]: 'Skipped', [LogState.LOG_STATE_FAILED]: 'Failed',
};

/** Run-local journal evidence, selected by program ID through the shared run-file route. */
export function Logs({ snapshot }: { snapshot: Snapshot }) {
  const { client } = useConsole();
  const programs = snapshot.logs?.programs ?? [];
  const counts = new Map<string, number>();
  for (const program of programs) counts.set(program.programId ?? '', (counts.get(program.programId ?? '') ?? 0) + 1);
  return <section className="panel logs"><h2>Logs</h2>
    <p>Exported files contain the entries still available in the host journal.</p>
    {snapshot.logsError ? <p role="alert">{snapshot.logsError}</p> : snapshot.logs?.programs.length ?
      <ul className="log-list">{snapshot.logs.programs.map((program, number) => <li key={number}>
        <div className="log-program"><div className="mono">{program.programId}</div>
          {program.programId && counts.get(program.programId) === 1 && (program.state === LogState.LOG_STATE_EXPORTED || program.state === LogState.LOG_STATE_FAILED && BigInt(String(program.sizeBytes ?? 0)) > 0n) &&
            <a className="button secondary" href={client.artifactUrl(snapshot.runId, RUN_FILE_ARTIFACT, { program: program.programId })} download aria-label={`Download ${program.state === LogState.LOG_STATE_FAILED ? 'partial log' : 'log'} for ${program.programId}`}>
              Download {program.state === LogState.LOG_STATE_FAILED ? 'partial log' : 'log'}
            </a>}
        </div>
        <span>{labels[program.state ?? 0] ?? 'Unknown'}</span>
        <span>{program.state === LogState.LOG_STATE_SKIPPED || program.state === LogState.LOG_STATE_PENDING ? '—' : bytesLabel(program.sizeBytes)}</span>
        {program.reason && <p className="log-reason">{program.reason}</p>}
      </li>)}</ul> : <p className="unavailable">Logs unavailable for this run.</p>}
  </section>;
}
