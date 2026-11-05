import { doramcap } from '../generated/console';

const telemetry = doramcap.telemetry.v1;
const NS_PER_MS = 1_000_000n;
const CORES = 4;
const ROOT_FLAGS = 17;
const LEAF_FLAGS = 1;
const ROOT_CGROUP = '1';
export const SESSION_ID = 'demo-session';
/** Ten samples per second, for a 15 s run. */
export const SAMPLE_MS = 100;
export const RUN_SAMPLES = 150;
const NS_PER_SAMPLE = BigInt(SAMPLE_MS) * NS_PER_MS;
/** The load changes twice per second, so the charts move without flickering. */
const SAMPLES_PER_LOAD_STEP = 5;

/** Busy fraction of one core for each group, cycled so the charts move. */
const LOAD = {
  app: { id: '11', path: '/docker/activity-lab', busy: [0.42, 0.55, 0.38, 0.61, 0.47] },
  stress: { id: '12', path: '/docker/load-generator', busy: [0.91, 0.84, 0.96, 0.88, 0.93] },
};
const groups = (heavy: boolean) => heavy ? [LOAD.app, LOAD.stress] : [LOAD.app];
const runtimeNs = (busy: number[], sequence: number) => {
  let total = 0;
  for (let sample = 1; sample <= sequence; sample++) total += busy[Math.floor(sample / SAMPLES_PER_LOAD_STEP) % busy.length];
  return BigInt(Math.round(total * 1000)) * NS_PER_SAMPLE / 1000n;
};
export const dictionaryEntries = (heavy: boolean) => groups(heavy).map(group => ({ id: group.id, parentId: ROOT_CGROUP, path: group.path }));

/** One real-format CPU packet; identical inputs give identical bytes. The first sample carries the names. */
export function cpuPacket(sequence: number, heavy: boolean, startedAtMs: number): Uint8Array {
  const leaves = groups(heavy);
  const leafRuntime = leaves.map(group => runtimeNs(group.busy, sequence));
  // Each group's time lands on one core, and the root counters total them.
  const rootRuntime = Array.from({ length: CORES }, (_, cpu) => leaves.reduce((sum, _group, index) => index % CORES === cpu ? sum + leafRuntime[index] : sum, 0n));
  return telemetry.Packet.encode(telemetry.Packet.fromObject({ snapshot: {
    version: 2, sessionId: SESSION_ID, source: 'demo',
    dictionary: sequence === 1 ? dictionaryEntries(heavy) : [],
    snapshot: {
      sequence: String(sequence),
      captureUnixNs: String(BigInt(startedAtMs) * NS_PER_MS + BigInt(sequence) * NS_PER_SAMPLE),
      monotonicEndNs: String(BigInt(sequence) * NS_PER_SAMPLE),
      records: [
        ...rootRuntime.map((runtime, cpu) => ({ cgroupId: ROOT_CGROUP, cpu, flags: ROOT_FLAGS, runtimeNs: String(runtime) })),
        ...leaves.map((group, index) => ({ cgroupId: group.id, parentId: ROOT_CGROUP, cpu: index % CORES, flags: LEAF_FLAGS, runtimeNs: String(leafRuntime[index]) })),
      ],
    },
  } })).finish();
}
