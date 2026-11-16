import { analysisRegistry } from '../../registry.js';
import type { AnalysisModule } from '../../module.js';
import type { CpuAnalysis } from './model.js';
import { analyzeCpuInWorker } from './worker-client.js';
import { compareCpu } from './compare.js';

const cpuModule: AnalysisModule<CpuAnalysis> = {
  id: 'cpu', label: 'CPU', analyze: analyzeCpuInWorker, compare: compareCpu,
};
analysisRegistry.register(cpuModule);
