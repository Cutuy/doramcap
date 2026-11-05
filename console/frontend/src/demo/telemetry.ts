import { doramcap } from '../generated/console';

const telemetry = doramcap.telemetry.v1;
const fault = doramcap.fault.v1;
const NS_PER_MS = 1_000_000n;
const CORES = 8;
const ROOT_FLAGS = 17;
const LEAF_FLAGS = 1;
const ROOT_CGROUP = '1';
const SESSION_ID = 'c4f1e29a7b3d4e60';
/** Ten samples per second, for a 7 s run. */
export const SAMPLE_MS = 100;
export const RUN_SAMPLES = 70;
const NS_PER_SAMPLE = BigInt(SAMPLE_MS) * NS_PER_MS;
/** The load changes twice per second, so the charts move without flickering. */
const SAMPLES_PER_LOAD_STEP = 5;
const scope = (container: string) => `/system.slice/docker-${container}.scope`;
/** Containers of the sample project and system apps, by busy fraction of one core; `worker` is cpu-burn's busy loop. */
const GROUPS = [
  { id: '4211', path: scope('8e1c5f02a7d94b36c0f1e8a25b7d3946ce10a8f7d2b6493e5c71f08ad4e92b67'), busy: [0.99, 0.98, 1, 0.99, 0.97, 1] },
  { id: '4187', path: scope('2b7d90c4e6f1385a9d02c7b4e1f8a63d50c9e27b14f6a83d7e05b2c9f4186da3'), busy: [0.05, 0.04, 0.07, 0.05, 0.06] },
  { id: '4163', path: scope('f4a26d18b9e0735c1a8f4d6e92b0c75e3d18f6a2049c7be5d3a1806f2c97e4b1'), busy: [0.03, 0.04, 0.03, 0.02, 0.04] },
  { id: '4142', path: scope('6c93e1a5f7082d4b6e1c9f3a5d70b28e4f16c3d9a7052e8b1f4d6a93c07e25d8'), busy: [0.02, 0.01, 0.03, 0.02] },
];
const runtimeNs = (busy: number[], sequence: number) => {
  let total = 0;
  for (let sample = 1; sample <= sequence; sample++) total += busy[Math.floor(sample / SAMPLES_PER_LOAD_STEP) % busy.length];
  return BigInt(Math.round(total * 1000)) * NS_PER_SAMPLE / 1000n;
};
const captureNs = (startedAtMs: number, sequence: number) => BigInt(startedAtMs) * NS_PER_MS + BigInt(sequence) * NS_PER_SAMPLE;

/** A real-format snapshot packet from each group's cumulative runtime. */
function snapshotPacket(sequence: number, startedAtMs: number, groups: { id: string }[], leafRuntime: bigint[]): Uint8Array {
  // Each group's time lands on one core, and the root counters total them.
  const rootRuntime = Array.from({ length: CORES }, (_, cpu) => groups.reduce((sum, _group, index) => index % CORES === cpu ? sum + leafRuntime[index] : sum, 0n));
  return telemetry.Packet.encode(telemetry.Packet.fromObject({ snapshot: {
    version: 2, sessionId: SESSION_ID, source: 'cpu-monitor',
    snapshot: {
      sequence: String(sequence),
      captureUnixNs: String(captureNs(startedAtMs, sequence)),
      monotonicEndNs: String(BigInt(sequence) * NS_PER_SAMPLE),
      records: [
        ...rootRuntime.map((runtime, cpu) => ({ cgroupId: ROOT_CGROUP, cpu, flags: ROOT_FLAGS, runtimeNs: String(runtime) })),
        ...groups.map((group, index) => ({ cgroupId: group.id, parentId: ROOT_CGROUP, cpu: index % CORES, flags: LEAF_FLAGS, runtimeNs: String(leafRuntime[index]) })),
      ],
    },
  } })).finish();
}
/** The built-in run's CPU snapshot packet; identical inputs give identical bytes. */
export function cpuPacket(sequence: number, startedAtMs: number): Uint8Array {
  return snapshotPacket(sequence, startedAtMs, GROUPS, GROUPS.map(group => runtimeNs(group.busy, sequence)));
}
/** Group names as the monitor publishes them on its dictionary topic. */
export function dictionaryPacket(startedAtMs: number, groups: { id: string; path: string }[] = GROUPS): Uint8Array {
  return telemetry.Packet.encode(telemetry.Packet.fromObject({ dictionary: {
    version: 1, sessionId: SESSION_ID, revision: '1', captureUnixNs: String(captureNs(startedAtMs, 0)),
    entries: groups.map(group => ({ id: group.id, parentId: ROOT_CGROUP, path: group.path })),
  } })).finish();
}
/** A Fault Watchdog snapshot: one connected source and no active conditions. */
export function faultPacket(): Uint8Array {
  return fault.AggregationPublication.encode(fault.AggregationPublication.fromObject({ version: 1, snapshot: {
    epoch: new Uint8Array(16).fill(0xa1), revision: '1', sources: [{ source: new Uint8Array(16).fill(0x5c), connected: true }],
  } })).finish();
}

/** A repeatable value in [0, 1) for one seed, position and stream. */
export function noise(seed: number, position: number, stream = 0): number {
  let mixed = (seed ^ Math.imul(position + 1, 0x9e3779b1) ^ Math.imul(stream + 1, 0x85ebca6b)) >>> 0;
  mixed = Math.imul(mixed ^ (mixed >>> 15), mixed | 1);
  mixed ^= mixed + Math.imul(mixed ^ (mixed >>> 7), mixed | 61);
  return ((mixed ^ (mixed >>> 14)) >>> 0) / 2 ** 32;
}
/** A repeatable 64-digit hex identifier derived from `text`. */
export function digest(text: string): string {
  const seed = [...text].reduce((hash, char) => Math.imul(hash ^ char.charCodeAt(0), 0x01000193) >>> 0, 0x811c9dc5);
  return Array.from({ length: 8 }, (_, part) => Math.floor(noise(seed, part) * 2 ** 32).toString(16).padStart(8, '0')).join('');
}

const LOAD_JITTER = 0.15, IDLE_BUSY = 0.01, MAX_TOTAL_BUSY = CORES * 0.97;
const SPIKE_SAMPLES = 3;
/** Visitor runs name groups after their services, so Top CPU consumers reads by service. */
const SERVICE_CGROUP_ROOT = '/system.slice/compose';
/** The sample project's services, by busy cores; a `spike` multiplies load every `period` samples. */
type LoadGroup = { name: string; busy: number; spike?: { period: number; factor: number } };
export const WORKLOAD_SERVICES: LoadGroup[] = [
  { name: 'object-detector', busy: 1.6, spike: { period: 11, factor: 1.9 } },
  { name: 'sensor-fusion', busy: 0.8 },
  { name: 'motion-planner', busy: 0.6, spike: { period: 17, factor: 2.4 } },
  { name: 'object-tracker', busy: 0.5 },
  { name: 'lidar-driver', busy: 0.4 },
  { name: 'camera-driver', busy: 0.3 },
  { name: 'vehicle-controller', busy: 0.2 },
];
const SYSTEM_BUSY: LoadGroup[] = [{ name: 'cpu-monitor', busy: 0.05 }, { name: 'live-bridge', busy: 0.03 }];

/** CPU telemetry for one run: services load the machine only while `running`. */
export function createWorkload(seed: number) {
  const services = WORKLOAD_SERVICES.length;
  const groups = [...WORKLOAD_SERVICES, ...SYSTEM_BUSY].map((group, index) => ({ ...group, id: String(5000 + index), path: `${SERVICE_CGROUP_ROOT}/${group.name}` }));
  const runtime = groups.map(() => 0);
  const load = (group: typeof groups[number], index: number, sequence: number, running: boolean) => {
    if (index < services && !running) return IDLE_BUSY;
    const { spike } = group;
    const spiking = spike && (sequence + Math.floor(noise(seed, index, 1) * spike.period)) % spike.period < SPIKE_SAMPLES;
    return group.busy * (spiking ? spike!.factor : 1) * (1 + LOAD_JITTER * (2 * noise(seed, sequence, index) - 1));
  };
  return {
    dictionary: (startedAtMs: number) => dictionaryPacket(startedAtMs, groups),
    packet(sequence: number, startedAtMs: number, running: boolean): Uint8Array {
      const busy = groups.map((group, index) => load(group, index, sequence, running));
      const scale = Math.min(1, MAX_TOTAL_BUSY / busy.reduce((sum, value) => sum + value, 0));
      busy.forEach((value, index) => { runtime[index] += value * scale * SAMPLE_MS * 1e6; });
      return snapshotPacket(sequence, startedAtMs, groups, runtime.map(value => BigInt(Math.round(value))));
    },
  };
}
