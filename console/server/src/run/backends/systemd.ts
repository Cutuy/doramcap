import { runPlan, type RunInputs } from '../run-plan.js';
import { lstat, readdir, realpath } from 'node:fs/promises';
import { join } from 'node:path';
import { compose, execution, systemd, type Config, type Project, type Run } from '../../native.js';
import { appBindings, type SelectedApp } from '../system-apps.js';

const UNIT_SUFFIX = '.service';

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

/** Generate system app programs from process settings and collect user app unit files.
 * Errors: missing launch inputs or a config target without a selected config.
 */
export async function systemdRequest(config: Config, run: RunInputs): Promise<Run> {
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
  const units = await projectUnits(run.project.directory!);
  return execution.RunRequest.create({
    run: runPlan(config, run),
    systemd: systemd.SystemdPlan.create({
      binaries,
      programs,
      unitFiles: units,
    }),
  });
}

/** Bind selected config, outputs and app files at the same targets as Compose. */
function appBinds(app: SelectedApp, run: RunInputs, dora: boolean) {
  const bindings = appBindings(app, dora ? join(run.artifacts, 'dora/runtime') : run.artifacts, run.relaySettings);
  if (app.configMount?.target && !bindings.config)
    throw new Error(`${app.id}: nothing to show at ${app.configMount.target}`);
  return [
    ...(bindings.config ? [bindings.config] : []),
    ...bindings.outputs,
    ...[...(app.files ?? []), ...(app.systemd?.files ?? [])].map((file) => ({ source: file.source, target: file.target!, readOnly: true })),
  ].map(({ source, target, readOnly }) => ({ source, target, readOnly }));
}
