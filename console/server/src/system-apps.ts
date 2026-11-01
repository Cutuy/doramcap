import { isAbsolute, join, relative, sep } from 'node:path';
import { lstat, realpath } from 'node:fs/promises';
import { compose, type App, type Project, type Run, type Output } from './native.js';
import { writeOnce } from './store.js';
import { LIVE_BRIDGE } from './live.js';

const INFRASTRUCTURE = [
  { id: 'dora-coordinator', label: 'Dora coordinator' },
  { id: 'dora-daemon', label: 'Dora IPC daemon' },
];
const child = (path: string) =>
  !!path && !isAbsolute(path) && path.split(/[\\/]/).every((part) => part && part !== '.' && part !== '..');
export function validateApps(apps: App[]) {
  const ids = new Set<string>(),
    outputs = new Set<string>();
  for (const app of apps) {
    if (
      !app.id ||
      !/^[-a-z0-9]{1,40}$/.test(app.id) ||
      ids.has(app.id) ||
      INFRASTRUCTURE.some((item) => item.id === app.id) ||
      app.id === LIVE_BRIDGE.id
    )
      throw new Error('empty, duplicate or reserved system app id');
    ids.add(app.id);
    if (!app.input || !isAbsolute(app.input.directory ?? '') || !app.input.files?.length)
      throw new Error('system app requires absolute directory and Compose files');
    if (app.graph && !isAbsolute(app.graph)) throw new Error('system app graph must be absolute');
    const sources = new Set<string>();
    for (const output of app.liveOutputs ?? []) {
      if (!/^[A-Za-z0-9_.-]{1,64}\/[A-Za-z0-9_.-]{1,64}$/.test(output.source ?? '') || sources.has(output.source!) || !output.kind)
        throw new Error('live output requires a unique node/output source and a kind');
      sources.add(output.source!);
    }
    for (const mount of app.outputMounts ?? [])
      if (!mount.service || !isAbsolute(mount.target ?? ''))
        throw new Error('output mount requires a service and absolute target');
    for (const output of app.recordings ?? []) {
      if (
        !output.id ||
        !/^[-a-zA-Z0-9]+$/.test(output.id) ||
        output.id === 'report' ||
        outputs.has(output.id)
      )
        throw new Error('empty, duplicate or reserved recording id');
      outputs.add(output.id);
      if (!child(output.path ?? '') || (output.stream && !child(output.stream)))
        throw new Error('recording paths must be relative beneath artifacts');
    }
  }
}
export function selectApps(apps: App[], requested?: string[]): App[] {
  const ids = requested ?? apps.filter((app) => app.defaultSelected).map((app) => app.id!);
  if (new Set(ids).size !== ids.length || ids.some((id) => !apps.some((app) => app.id === id)))
    throw new Error('InvalidInput');
  return ids.map((id) => apps.find((app) => app.id === id)!);
}
/** `live` is undefined when no bridge is configured. */
export function appOptions(apps: App[], selected: App[], live?: boolean) {
  return [
    ...INFRASTRUCTURE.map((app) => ({ ...app, selected: true, required: true })),
    ...(live === undefined ? [] : [{ ...LIVE_BRIDGE, selected: live, required: false }]),
    ...apps.map((app) => ({
      id: app.id,
      label: app.label,
      selected: selected.includes(app),
      required: false,
    })),
  ];
}
export function outputs(apps: App[]): Output[] {
  return apps.flatMap((app) => app.recordings ?? []);
}
export async function bindOutputs(apps: App[], run: Run, workspace: string, stage: string) {
  run.systemApps = [];
  for (const app of apps) {
    const input = compose.ComposeProject.decode(compose.ComposeProject.encode(app.input!).finish());
    input.key = app.id!;
    if (app.outputMounts?.length) {
      const services: Record<string, { volumes: unknown[] }> = {};
      for (const mount of app.outputMounts) {
        (services[mount.service!] ??= { volumes: [] }).volumes.push({
          type: 'bind',
          source: run.artifacts,
          target: mount.target,
          bind: { create_host_path: false },
        });
      }
      const path = join(workspace, `${stage}-${app.id}.json`);
      await writeOnce(path, Buffer.from(JSON.stringify({ services })));
      input.files.push(path);
    }
    run.systemApps.push(input);
  }
}
/** Discover native Compose inputs beneath the approved project root. */
export async function discover(root: string, directory: string, profiles: string[]): Promise<Project> {
  if (!isAbsolute(directory)) throw new Error('InvalidInput');
  const actual = await realpath(directory),
    approved = await realpath(root),
    inside = relative(approved, actual);
  if (
    inside === '..' ||
    inside.startsWith('..' + sep) ||
    isAbsolute(inside) ||
    !(await lstat(actual)).isDirectory()
  )
    throw new Error('InvalidInput');
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
