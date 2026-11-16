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
    const id = run.detail?.entry?.runId ?? run.current?.runId ?? '';
    const setup = run.directory && id ? readSetupRecord(run.directory, id, '') : null;
    const capture = execution.RunCapture.create(run.capture ?? {});
    const detail = run.detail ? wire.HistoryDetail.create(run.detail) : null;
    signal.throwIfAborted();
    if (run.current) return analyzeSnapshot('review', { setup, capture, profiles: run.current.profiles ?? [],
      apps: run.current.systemApps as never, writePolicy: run.current.writePolicy,
    });
    return analyzeSnapshot(detail ? 'history' : 'machine', { setup, capture, detail,
      profiles: run.detail?.entry?.profiles ?? [], apps: run.detail?.entry?.systemApps as never, writePolicy: run.detail?.status?.writePolicy,
    });
  },
  compare: compareAnalyses,
};
