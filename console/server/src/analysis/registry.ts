import { wire } from '../native.js';
import { createAnalysisTask, type AnalysisModule } from './module.js';

/** Registered definitions are stateless; per-run inputs and live tasks belong to the caller. */
export class AnalysisRegistry {
  private modules = new Map<string, ReturnType<typeof createAnalysisTask>>();
  register<Analysis>(module: AnalysisModule<Analysis>): void {
    if (!module.id || this.modules.has(module.id)) throw new Error('Duplicate or empty analysis identity: ' + module.id);
    this.modules.set(module.id, createAnalysisTask(module));
  }
  list() { return [...this.modules.values()].map(({ id, label }) => wire.AnalysisModule.create({ id, label })); }
  get(id: string) { return this.modules.get(id); }
}

/** Built-in imports register definitions here; tests can use an isolated registry. */
export const analysisRegistry = new AnalysisRegistry();
