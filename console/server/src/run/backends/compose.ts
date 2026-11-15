import { lstat, realpath } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { compose, composePlan, execution, type Config, type Project, type Run } from '../../native.js';
import { appBindings, type SelectedApp } from '../system-apps.js';
import { runPlan, type RunInputs } from '../run-plan.js';
import { writeOnce } from '../../files.js';

/** Compose system app that keeps the run IPC namespace available until shutdown. */
export const RUN_SHM_APP = 'doramcap-shm';

/** Build native Compose inputs, including selected app mounts and the run's IPC helper. */
export async function composeRequest(config: Config, run: RunInputs): Promise<Run> {
  const apps: SelectedApp[] = [...run.apps, ...config.apps.filter((app) => app.id === RUN_SHM_APP)];
  const projects: Project[] = [run.project, ...config.apps.filter((app) => config.doraApps.includes(app.id!))
    .map((app) => ({ ...app.input!, key: app.id! }))];
  for (const app of apps) {
    const input = compose.ComposeProject.decode(compose.ComposeProject.encode(app.input!).finish());
    input.key = app.id!;
    const bindings = appBindings(app, run.artifacts, run.relaySettings);
    await composeBindings(input, [
      ...bindings.outputs, ...(app.profile && bindings.config ? [bindings.config] : []),
    ], join(run.workspace, `${run.stage}-${app.id}.json`));
    if (app.liveRelay && bindings.config)
      await composeBindings(input, [bindings.config], join(run.workspace, `${run.stage}-${app.id}-relay.json`));
    projects.push(input);
  }
  return execution.RunRequest.create({
    run: runPlan(config, { ...run, apps }),
    compose: composePlan.ComposePlan.create({ projects }),
  });
}

/** Append a native overlay; the relay config follows the app's output overlay. */
async function composeBindings(input: Project, bindings: ReturnType<typeof appBindings>['outputs'], path: string) {
  if (!bindings.length) return;
  const services: Record<string, { volumes: unknown[] }> = {};
  for (const binding of bindings) {
    (services[binding.service!] ??= { volumes: [] }).volumes.push({
      type: 'bind', source: binding.source, target: binding.target,
      ...(binding.readOnly ? { read_only: true } : {}),
      bind: { create_host_path: false },
    });
  }
  await writeOnce(path, Buffer.from(JSON.stringify({ services })));
  input.files!.push(path);
}

/** Discover the native Compose inputs of one project directory. */
export async function composeProject(directory: string, profiles: string[]): Promise<Project> {
  if (!isAbsolute(directory)) throw new Error('InvalidInput');
  const actual = await realpath(directory);
  if (!(await lstat(actual)).isDirectory()) throw new Error('InvalidInput');
  const exists = async (name: string) => {
    try {
      if (!(await lstat(join(actual, name))).isFile()) throw new Error('non-regular Compose input');
      return true;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === 'ENOENT') return false;
      throw error;
    }
  };
  const bases = [];
  for (const name of ['compose.yaml', 'compose.yml', 'docker-compose.yaml', 'docker-compose.yml'])
    if (await exists(name)) bases.push(name);
  if (bases.length !== 1) throw new Error('exactly one default Compose file is required');
  const prefix = bases[0].startsWith('docker-compose.') ? 'docker-compose' : 'compose',
    files = [bases[0]];
  for (const ext of ['yaml', 'yml'])
    if (await exists(`${prefix}.override.${ext}`)) files.push(`${prefix}.override.${ext}`);
  if (files.length > 2) throw new Error('ambiguous Compose overrides');
  return compose.ComposeProject.create({
    key: 'workload',
    directory: actual,
    files,
    envFiles: (await exists('.env')) ? ['.env'] : [],
    profiles,
    removeVolumes: false,
  });
}
