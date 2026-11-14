import { runPlan } from './run-plan.js';
import { lstat, readdir, realpath } from 'node:fs/promises';
import { join } from 'node:path';
import { compose, execution, systemd, type Config, type Project } from './native.js';
import { type SelectedApp } from './system-apps.js';

const UNIT_SUFFIX = '.service';

export type RunRequest = InstanceType<typeof execution.RunRequest>;

/** Units a project contributes unchanged: every `*.service` at its root, sorted by name. */
export async function projectUnits(project: string) {
  return (await readdir(project, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith(UNIT_SUFFIX))
    .map((entry) => join(project, entry.name))
    .sort();
}

/** The project directory as a run's project; its units are its `*.service` files. */
export async function systemdProject(directory: string): Promise<Project> {
  const actual = await realpath(directory);
  if (!(await lstat(actual)).isDirectory()) throw new Error('InvalidInput');
  return compose.ComposeProject.create({ key: 'workload', directory: actual });
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

/** Generate system app programs from process settings and collect user app unit files.
 * Errors: missing launch inputs or a config target without a selected config.
 */
export async function systemdPlan(config: Config, run: SystemdRun): Promise<RunRequest> {
  const binaries = config.binaries!;
  const apps = [...config.apps.filter((app) => config.doraApps.includes(app.id!)), ...run.apps];
  const programs = apps.map((app) => {
    if (!app.process) throw new Error(`${app.id}: system app needs process settings for systemd`);
    const process = app.process;
    if (!process.argv?.length || !/^[^/\\]+$/.test(process.argv[0]) || ['.', '..'].includes(process.argv[0]))
      throw new Error(`${app.id}: process needs an executable basename`);
    return systemd.Program.create({
      id: app.id!, argv: [join(binaries, process.argv[0]), ...process.argv.slice(1)],
      workingDirectory: process.workingDirectory, memlockBytes: process.memlockBytes,
      cpus: process.cpus, killSignal: process.stopSignal, stopSeconds: process.stopSeconds,
      binds: appBinds(app, run, config.doraApps.includes(app.id!)),
    });
  });
  const units = await projectUnits(run.project);
  return execution.RunRequest.create({
    run: runPlan(config, run, run.graph),
    systemd: systemd.SystemdPlan.create({
      binaries,
      programs,
      unitFiles: units,
    }),
  });
}

/** Bind selected config, outputs and app files at the same targets as Compose. */
function appBinds(app: SelectedApp, run: SystemdRun, dora: boolean) {
  const target = app.configMount?.target;
  const config = app.profile?.config ?? (app.liveRelay ? run.relaySettings : undefined);
  if (target && !config) throw new Error(`${app.id}: nothing to show at ${target}`);
  return [
    ...(target ? [{ source: config, target, readOnly: true }] : []),
    ...(app.outputMounts ?? []).map((mount) => ({ source: dora ? join(run.artifacts, 'dora/runtime') : run.artifacts, target: mount.target, readOnly: false })),
    ...[...(app.files ?? []), ...(app.systemd?.files ?? [])].map((file) => ({ source: file.source, target: file.target, readOnly: true })),
  ];
}
