import { mkdir, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { availablePort } from './launch.js';
import { execution, launch, systemd, type Config } from './native.js';
import { outputs, type SelectedApp } from './system-apps.js';

/** Smallest locked-memory limit granted to the Dora runtime, as for Compose. */
const MIN_DORA_MEMLOCK_BYTES = 64 * 1024 * 1024;
/** Dora's executable inside the binaries directory. */
const DORA_EXECUTABLE = 'dora';
const UNIT_SUFFIX = '.service';

export type RunRequest = InstanceType<typeof execution.RunRequest>;

/** Units a project contributes unchanged: every `*.service` at its root, sorted by name. */
export async function projectUnits(project: string) {
  return (await readdir(project, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith(UNIT_SUFFIX))
    .map((entry) => join(project, entry.name))
    .sort();
}

/** Inputs Console has already resolved for one run. */
export interface SystemdRun {
  id: string;
  project: string;
  artifacts: string;
  workspace: string;
  stage: string;
  apps: SelectedApp[];
  /** Merged Dora graph; empty registers none. */
  graph: string;
  /** Generated live-bridge settings, shown at the relay's config target. */
  relaySettings?: string;
}

/** Two distinct free loopback ports for the Dora coordinator and daemon. */
async function doraPorts() {
  const coordinator = await availablePort();
  let daemon = await availablePort();
  while (daemon === coordinator) daemon = await availablePort();
  return { coordinator, daemon };
}

/**
 * Build the run request with a systemd backend: Dora, then the selected system apps,
 * then the project's units.
 * Creates the private Dora runtime directory. Errors: an app without a program executable name,
 * or a config target with no config to show there.
 */
export async function systemdPlan(config: Config, run: SystemdRun): Promise<RunRequest> {
  const binaries = config.binaries!;
  const runtime = join(run.workspace, `${run.stage}-dora`);
  await mkdir(runtime, { recursive: true, mode: 0o700 });
  const programs = run.apps.map((app) => program(binaries, app, run));
  const units = await projectUnits(run.project);
  const recordings = outputs(run.apps).map((output) => ({
    recording: join(run.artifacts, output.path!),
    stream: output.stream ? join(run.artifacts, output.stream) : '',
  }));
  const ports = await doraPorts();
  return execution.RunRequest.create({
    run: launch.RunPlan.create({
      id: run.id,
      repositoryRoot: run.project,
      artifacts: run.artifacts,
      recordings,
      readinessSeconds: config.runtime!.readinessSeconds!,
      stopSeconds: config.runtime!.stopSeconds!,
    }),
    systemd: systemd.SystemdPlan.create({
      dora: {
        executable: join(binaries, DORA_EXECUTABLE),
        coordinatorPort: ports.coordinator,
        daemonPort: ports.daemon,
        graph: run.graph,
        runtimeDirectory: runtime,
        memlockBytes: Math.max(Number(config.dora?.memlockBytes ?? 0), MIN_DORA_MEMLOCK_BYTES),
      },
      groups: [
        ...(programs.length ? [{ programs, awaitRecordings: recordings.length > 0 }] : []),
        ...(units.length ? [{ unitFiles: units }] : []),
      ],
    }),
  });
}

/** One system app's unit: its executable, its chosen config and outputs at container paths. */
function program(binaries: string, app: SelectedApp, run: SystemdRun) {
  const facts = app.program;
  if (!facts?.executable || facts.executable.includes('/'))
    throw new Error(`${app.id}: SYSTEMD needs a program executable name`);
  const config = app.profile?.config ?? (app.liveRelay ? run.relaySettings : undefined);
  if (facts.configTarget && !config) throw new Error(`${app.id}: nothing to show at ${facts.configTarget}`);
  return {
    id: app.id,
    argv: [join(binaries, facts.executable), ...(facts.args ?? [])],
    memlockBytes: facts.memlockBytes,
    killSignal: facts.killSignal,
    binds: [
      ...(facts.configTarget ? [{ source: config, target: facts.configTarget, readOnly: true }] : []),
      ...(facts.outputTargets ?? []).map((target) => ({ source: run.artifacts, target, readOnly: false })),
      ...(facts.files ?? []).map((file) => ({ source: file.source, target: file.target, readOnly: true })),
    ],
  };
}
