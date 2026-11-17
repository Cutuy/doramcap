import { analysisRegistry } from '../registry.js';
import { runModule } from './run/module.js';
import { machineModule } from './machine/module.js';

analysisRegistry.register(runModule);
analysisRegistry.register(machineModule);
