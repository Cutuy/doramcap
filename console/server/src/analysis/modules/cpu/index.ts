import type { AnalysisModule } from '../../module.js';
import type { CpuAnalysis } from './model.js';
import { analyzeCpuInWorker } from './worker-client.js';
import { compareCpu } from './compare.js';

export const cpuModule: AnalysisModule<CpuAnalysis> = {
  id: 'cpu', label: 'CPU', analyze: analyzeCpuInWorker, compare: compareCpu,
};
