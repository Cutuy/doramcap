import { analysisRegistry } from '../registry.js';
import { runModule } from './run/module.js';
import { machineModule } from './machine/module.js';
import { cpuModule } from './cpu/index.js';

analysisRegistry.register(runModule);
analysisRegistry.register(machineModule);
analysisRegistry.register(cpuModule);
