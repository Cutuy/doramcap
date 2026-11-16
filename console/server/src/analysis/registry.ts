import { wire } from '../native.js';
import type { AnalysisModule, AnalysisRun } from './module.js';

/** Registered definitions are stateless; per-run inputs and live tasks belong to the caller. */
export class AnalysisRegistry {
  private modules = new Map<string, ReturnType<typeof createAnalysisTask>>();
  /** Add one definition; empty or duplicate identities throw before registration. */
  register<Analysis>(module: AnalysisModule<Analysis>): void {
    if (!module.id || this.modules.has(module.id)) throw new Error('Duplicate or empty analysis identity: ' + module.id);
    this.modules.set(module.id, createAnalysisTask(module));
  }
  /** Registered identities and labels in registration order; never invokes an analyzer. */
  list() { return [...this.modules.values()].map(({ id, label }) => wire.AnalysisModule.create({ id, label })); }
  /** Return the selected task definition, or undefined for an unknown identity. */
  get(id: string) { return this.modules.get(id); }
}

/** Analyze both runs concurrently; failure or cancellation waits for both analyzers' cleanup. */
function createAnalysisTask<Analysis>(module: AnalysisModule<Analysis>) {
  return {
    id: module.id, label: module.label,
    async run(left: AnalysisRun, right: AnalysisRun, signal: AbortSignal) {
      signal.throwIfAborted();
      const cancellation = new AbortController();
      const taskSignal = AbortSignal.any([signal, cancellation.signal]);
      const tasks = [left, right].map(async run => {
        try {
          taskSignal.throwIfAborted();
          return await module.analyze(run, taskSignal);
        } catch (error) {
          cancellation.abort(error);
          throw error;
        }
      });
      try {
        const [a, b] = await Promise.all(tasks);
        taskSignal.throwIfAborted();
        const rows = module.compare(a, b);
        taskSignal.throwIfAborted();
        return wire.AnalysisResponse.create({ moduleId: module.id, label: module.label, rows, truncated: false });
      } finally {
        await Promise.allSettled(tasks);
      }
    },
  };
}

/** Built-in imports register definitions here; tests can use an isolated registry. */
export const analysisRegistry = new AnalysisRegistry();
