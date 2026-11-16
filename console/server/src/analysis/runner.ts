import type { AnalysisRun } from './module.js';
import type { AnalysisRegistry } from './registry.js';
import type { wire } from '../native.js';

/** One service owns its active invocations and awaits cleanup before shutdown completes. */
export class AnalysisRunner {
  private tasks = new Map<AbortController, Promise<typeof wire.AnalysisResponse.prototype>>();
  private closing?: Promise<void>;
  constructor(private registry: AnalysisRegistry) {}
  async run(id: string, left: AnalysisRun, right: AnalysisRun, signal: AbortSignal) {
    signal.throwIfAborted();
    if (this.closing) throw new Error('Closing');
    const module = this.registry.get(id);
    if (!module) throw new Error('InvalidInput');
    const controller = new AbortController(), abort = () => controller.abort(signal.reason);
    signal.addEventListener('abort', abort, { once: true });
    if (signal.aborted) abort();
    const task = module.run(left, right, controller.signal);
    this.tasks.set(controller, task);
    try { return await task; }
    finally { signal.removeEventListener('abort', abort); this.tasks.delete(controller); }
  }
  close(): Promise<void> {
    if (!this.closing) {
      const tasks = [...this.tasks.values()];
      this.closing = Promise.allSettled(tasks).then(() => {});
      for (const controller of this.tasks.keys()) controller.abort();
    }
    return this.closing;
  }
}
