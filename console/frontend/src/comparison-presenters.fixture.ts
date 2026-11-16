import { runFactLabel, runFactValue, comparedRunValue } from './analysis/modules/run/presenter';
import { machineFactLabel, machineFactValue, comparedMachineValue } from './analysis/modules/machine/presenter';

const run = (name: string) => name === 'write_policy' || name === 'recording_bytes' || name === 'duration_seconds' || name === 'source_commit' || name.startsWith('image.');
export const factLabel = (name: string) => run(name) ? runFactLabel(name) : machineFactLabel(name);
export const factValue = (name: string, value: string) => run(name) ? runFactValue(name,value) : machineFactValue(name,value);
export const comparedFactValue = (name: string, value: string, others: string[]) => run(name) ? comparedRunValue(name,value,others) : comparedMachineValue(name,value,others);
