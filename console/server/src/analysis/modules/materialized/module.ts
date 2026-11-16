import { execution, wire } from '../../../native.js';
import type { AnalysisModule } from '../../module.js';
import { readSetupRecord } from '../../../records/setup.js';
import { analyzeSnapshot, compareAnalyses, type MaterializedAnalysis } from './compare.js';

export const MATERIALIZED_MODULE_ID = 'materialized';
/** Default analysis of saved facts and current observations; no recording scan is required. */
export const materializedModule: AnalysisModule<MaterializedAnalysis> = {
  id: MATERIALIZED_MODULE_ID, label: 'Saved facts',
  async analyze(run, signal) {
    signal.throwIfAborted();
    if (run.snapshotCurrent && (run.snapshotHistory || run.snapshotCurrent.phase !== wire.Phase.REVIEWED)) throw new Error('InvalidInput');
    const id = run.snapshotHistory?.entry?.runId ?? run.snapshotCurrent?.runId ?? '';
    const setup = run.directory && id ? readSetupRecord(run.directory, id, '') : null;
    const capture = execution.RunCapture.create(run.capture ?? {});
    const detail = run.snapshotHistory ? wire.HistoryDetail.create(run.snapshotHistory) : null;
    signal.throwIfAborted();
    if (run.snapshotCurrent) return analyzeSnapshot('review', { setup, capture, profiles: run.snapshotCurrent.profiles ?? [],
      apps: run.snapshotCurrent.systemApps as never, writePolicy: run.snapshotCurrent.writePolicy,
    });
    return analyzeSnapshot(detail ? 'history' : 'machine', { setup, capture, detail,
      profiles: run.snapshotHistory?.entry?.profiles ?? [], apps: run.snapshotHistory?.entry?.systemApps as never, writePolicy: run.snapshotHistory?.status?.writePolicy,
    });
  },
  compare: compareAnalyses,
};
