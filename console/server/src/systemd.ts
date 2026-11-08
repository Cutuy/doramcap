import { runPlan } from './run-plan.js';
import { readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { execution, systemd, type Config } from './native.js';
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

/**
 * Build the run request with a systemd backend: Dora, then the selected system apps'
 * unit files, then the project's units. Launch details remain in native app files.
 * Errors: an app without a unit file, or a config target with no config to show there.
 */
export async function systemdPlan(config: Config, run: SystemdRun): Promise<RunRequest> {
  const binaries = config.binaries!;
  const appUnits = [...config.apps.filter((app) => config.doraApps.includes(app.id!)), ...run.apps].map((app) => appUnit(app, run));
  const units = await projectUnits(run.project);
  return execution.RunRequest.create({
    run: runPlan(config, run, run.graph),
    systemd: systemd.SystemdPlan.create({
      binaries,
      appUnits,
      unitFiles: units,
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
