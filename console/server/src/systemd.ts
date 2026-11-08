import { mkdir, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { availablePort } from './launch.js';
import { execution, launch, systemd, type Config } from './native.js';
import { outputs, type SelectedApp } from './system-apps.js';

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
 * Build the run request with a systemd backend: Dora, then the selected system apps'
 * unit files, then the project's units. Creates the private Dora runtime directory.
 * Errors: an app without a unit file, or a config target with no config to show there.
 */
export async function systemdPlan(config: Config, run: SystemdRun): Promise<RunRequest> {
  const binaries = config.binaries!;
  const runtime = join(run.workspace, `${run.stage}-dora`);
  await mkdir(runtime, { recursive: true, mode: 0o700 });
  const appUnits = run.apps.map((app) => appUnit(app, run));
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
        memlockBytes: config.dora?.memlockBytes ?? 0,
      },
      binaries,
      groups: [
        ...(appUnits.length ? [{ appUnits, awaitRecordings: recordings.length > 0 }] : []),
        ...(units.length ? [{ unitFiles: units }] : []),
      ],
    }),
  });
}

/** One system app's unit file, with its chosen config and outputs bound at its targets. */
function appUnit(app: SelectedApp, run: SystemdRun) {
  if (!app.systemd?.unit) throw new Error(`${app.id}: SYSTEMD needs a unit file`);
  const target = app.configMount?.target;
  const config = app.profile?.config ?? (app.liveRelay ? run.relaySettings : undefined);
  if (target && !config) throw new Error(`${app.id}: nothing to show at ${target}`);
  return {
    file: app.systemd.unit,
    binds: [
      ...(target ? [{ source: config, target, readOnly: true }] : []),
      ...(app.outputMounts ?? []).map((mount) => ({ source: run.artifacts, target: mount.target, readOnly: false })),
      ...(app.systemd.files ?? []).map((file) => ({ source: file.source, target: file.target, readOnly: true })),
    ],
  };
}
