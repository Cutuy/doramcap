import { isAbsolute, join } from 'node:path';
import { lstat, realpath } from 'node:fs/promises';
import { compose, type App, type Project, type Run, type Output } from './native.js';
import { writeOnce } from './files.js';

const INFRASTRUCTURE = [
  { id: 'dora-coordinator', label: 'Dora coordinator' },
  { id: 'dora-daemon', label: 'Dora IPC daemon' },
];
const child = (path: string) =>
  !!path && !isAbsolute(path) && path.split(/[\\/]/).every((part) => part && part !== '.' && part !== '..');
/** Reject invalid live and recording outputs, adding recording IDs to `seen`. */
function validateOutputs(recordings: Output[], live: NonNullable<App['liveOutputs']>, seen: Set<string>) {
  const sources = new Set<string>();
  for (const output of live) {
    if (!/^[A-Za-z0-9_.-]{1,64}\/[A-Za-z0-9_.-]{1,64}$/.test(output.source ?? '') || sources.has(output.source!) || !output.kind)
      throw new Error('live output requires a unique node/output source and a kind');
    sources.add(output.source!);
  }
  for (const output of recordings) {
    if (!output.id || !/^[-a-zA-Z0-9]+$/.test(output.id) || output.id === 'report' || seen.has(output.id))
      throw new Error('empty, duplicate or reserved recording id');
    seen.add(output.id);
    if (!child(output.path ?? '') || (output.stream && !child(output.stream)))
      throw new Error('recording paths must be relative beneath artifacts');
  }
}
export function validateApps(apps: App[]) {
  const ids = new Set<string>(),
    outputs = new Set<string>();
  let relayed = false;
  for (const app of apps) {
    if (
      !app.id ||
      !/^[-a-z0-9]{1,40}$/.test(app.id) ||
      ids.has(app.id) ||
      INFRASTRUCTURE.some((item) => item.id === app.id)
    )
      throw new Error('empty, duplicate or reserved system app id');
    ids.add(app.id);
    if (!app.input || !isAbsolute(app.input.directory ?? '') || !app.input.files?.length)
      throw new Error('system app requires absolute directory and Compose files');
    if (app.graph && !isAbsolute(app.graph)) throw new Error('system app graph must be absolute');
    if (app.liveRelay) {
      if (relayed) throw new Error('only one system app may set live_relay');
      relayed = true;
      if (!app.configMount?.service || !isAbsolute(app.configMount.target ?? '') || app.graph || app.launchProfiles?.length || app.liveOutputs?.length)
        throw new Error('live_relay requires a config mount and excludes graph, launch profiles and live outputs');
      if ((app.liveRelay.port ?? 0) > 65535) throw new Error('live_relay port must be in 0..65535');
    }
    for (const mount of app.outputMounts ?? [])
      if (!mount.service || !isAbsolute(mount.target ?? ''))
        throw new Error('output mount requires a service and absolute target');
    validateOutputs(app.recordings ?? [], app.liveOutputs ?? [], outputs);
    const profiles = app.launchProfiles ?? [];
    if (profiles.length && (!app.configMount?.service || !isAbsolute(app.configMount.target ?? '')))
      throw new Error('launch profiles require a config mount with a service and absolute target');
    const profileIds = new Set<string>(),
      shared = new Set(outputs);
    for (const profile of profiles) {
      if (!/^[-a-z0-9]{1,40}$/.test(profile.id ?? '') || profileIds.has(profile.id!) || !profile.label || !isAbsolute(profile.config ?? ''))
        throw new Error('launch profile requires a unique id, a label and an absolute config');
      if (profile.graph && !isAbsolute(profile.graph)) throw new Error('profile graph must be absolute');
      profileIds.add(profile.id!);
      // Only one profile launches, so profiles of one app may reuse recording IDs.
      const seen = new Set(shared);
      validateOutputs(profile.recordings ?? [], [...(app.liveOutputs ?? []), ...(profile.liveOutputs ?? [])], seen);
      seen.forEach((id) => outputs.add(id));
    }
  }
}
/** An app with its launch profile's outputs applied. */
export type SelectedApp = App & { profile?: NonNullable<App['launchProfiles']>[number] };
export function selectApps(apps: App[], requested?: string[], profiles: Record<string, string> = {}): SelectedApp[] {
  const ids = requested ?? apps.filter((app) => app.defaultSelected).map((app) => app.id!);
  if (
    new Set(ids).size !== ids.length ||
    ids.some((id) => !apps.some((app) => app.id === id)) ||
    Object.keys(profiles).some((id) => !ids.includes(id))
  )
    throw new Error('InvalidInput');
  return ids.map((id) => {
    const app = apps.find((app) => app.id === id)!,
      options = app.launchProfiles ?? [];
    if (!options.length && !(id in profiles)) return app;
    const profile = id in profiles ? options.find((option) => option.id === profiles[id]) : options[0];
    if (!profile) throw new Error('InvalidInput');
    return {
      ...app,
      profile,
      recordings: [...(app.recordings ?? []), ...(profile.recordings ?? [])],
      liveOutputs: [...(app.liveOutputs ?? []), ...(profile.liveOutputs ?? [])],
    };
  });
}
/** App IDs and launch profiles for one selection, as stored with a run. */
export const appProfiles = (apps: SelectedApp[]) =>
  Object.fromEntries(apps.flatMap((app) => (app.profile ? [[app.id!, app.profile.id!]] : [])));
export function appOptions(apps: App[], selected: SelectedApp[]) {
  return [
    ...INFRASTRUCTURE.map((app) => ({ ...app, selected: true, required: true })),
    ...apps.map((app) => {
      const chosen = selected.find((item) => item.id === app.id),
        options = app.launchProfiles ?? [];
      return {
        id: app.id,
        label: app.label,
        selected: !!chosen,
        required: false,
        profiles: options.map(({ id, label }) => ({ id, label })),
        profile: chosen?.profile?.id ?? options[0]?.id ?? '',
      };
    }),
  ];
}
export function outputs(apps: App[]): Output[] {
  return apps.flatMap((app) => app.recordings ?? []);
}
/** Services of apps that record; each writes through the io_uring MCAP writer. */
export function mcapServices(apps: App[]): string[] {
  return [...new Set(apps.filter((app) => app.recordings?.length).flatMap((app) => (app.outputMounts ?? []).map((mount) => mount.service!)))];
}
export async function bindOutputs(apps: SelectedApp[], run: Run, workspace: string, stage: string) {
  run.systemApps = [];
  for (const app of apps) {
    const input = compose.ComposeProject.decode(compose.ComposeProject.encode(app.input!).finish());
    input.key = app.id!;
    if (app.outputMounts?.length || app.profile) {
      const services: Record<string, { volumes: unknown[] }> = {};
      for (const mount of app.outputMounts ?? []) {
        (services[mount.service!] ??= { volumes: [] }).volumes.push({
          type: 'bind',
          source: run.artifacts,
          target: mount.target,
          bind: { create_host_path: false },
        });
      }
      if (app.profile)
        (services[app.configMount!.service!] ??= { volumes: [] }).volumes.push({
          type: 'bind',
          source: app.profile.config,
          target: app.configMount!.target,
          read_only: true,
          bind: { create_host_path: false },
        });
      const path = join(workspace, `${stage}-${app.id}.json`);
      await writeOnce(path, Buffer.from(JSON.stringify({ services })));
      input.files.push(path);
    }
    run.systemApps.push(input);
  }
}
/** Discover the native Compose inputs of one project directory. */
export async function discover(directory: string, profiles: string[]): Promise<Project> {
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
