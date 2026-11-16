import { copyFile, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execution, wire, type NativeExecution } from '../src/native.js';
import { ConsoleService } from '../src/service.js';
import { Store } from '../src/records/store.js';
import { History } from '../src/records/history.js';
import { cpuRecording, dictionary, snapshot } from './cpu-fixture.js';

export const comparisonRuns = ['a'.repeat(32), 'b'.repeat(32)] as const;
export const comparisonParent = 'doramcap-compose-0123456789ab-fedcba987654-user-worker.slice';
export const comparisonCgroup = '/doramcap.slice/doramcap-compose.slice/doramcap-compose-0123456789ab.slice/' +
  'doramcap-compose-0123456789ab-fedcba987654.slice/doramcap-compose-0123456789ab-fedcba987654-user.slice/' + comparisonParent;
export const comparisonSelection = comparisonRuns.map((runId, index) => ({ label: index ? 'B' : 'A', runIds: [runId] }));

/** Real saved runs, database, native safe opens and optional production-built service/worker. */
export async function comparisonFixture(production = false, packets?: Uint8Array[] | { recording: string }) {
  const root = await mkdtemp(join(tmpdir(), 'console-analysis-'));
  const store = await Store.open(join(root, 'data'));
  const engine: NativeExecution = {
    composeProfiles: async () => execution.ComposeProfiles.create(), inspect: async () => execution.Inspection.create(),
    start: async () => {}, stop: async () => {}, poll: () => execution.ExecutionUpdate.create(), close: async () => {},
  };
  const Service = production ? (await import(new URL('../dist/service.js', import.meta.url).href)).ConsoleService as typeof ConsoleService : ConsoleService;
  const config = wire.ConsoleConfig.create({ projectRoot: root, dataRoot: store.root, apps: [] });
  const backend = new Service(config, engine, store);
  for (const [index, runId] of comparisonRuns.entries()) {
    const directory = join(store.root, 'runs', runId);
    await mkdir(directory, { recursive: true });
    await writeFile(join(directory, 'experiment.input.json'), JSON.stringify({ services: { worker: { cgroup_parent: comparisonParent } } }));
    if (index === 0 && packets && !Array.isArray(packets)) await copyFile(packets.recording, join(directory, 'cpu.mcap'));
    else await writeFile(join(directory, 'cpu.mcap'), await cpuRecording(index === 0 && Array.isArray(packets) ? packets : [
      dictionary(comparisonCgroup), snapshot(1, 1_000_000_000n, [0n]), snapshot(2, 2_000_000_000n, [BigInt(index + 1) * 1_000_000_000n]),
    ]));
    new History(store).record(wire.Status.fromObject({ runId, phase: wire.Phase.FINISHED,
      recordings: [{ id: 'cpu', path: 'cpu.mcap', cpu: true }], outcome: { execution: 'Completed', recordingFinalized: true, boundaryClean: true,
        serviceExits: [{ service: 'worker', exitCode: index ? 0 : 137, killed: !index }] } }), 1, 2);
  }
  return { root, store, backend, firstRecording: join(store.root, 'runs', comparisonRuns[0], 'cpu.mcap'),
    close: async () => { await backend.close(); await rm(root, { recursive: true, force: true }); } };
}
