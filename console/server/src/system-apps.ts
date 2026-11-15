import { isAbsolute } from 'node:path';
import { launch, type App, type Output } from './native.js';

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
export function validateApps(apps: App[], form = launch.Form.COMPOSE) {
  const composeForm = form !== launch.Form.SYSTEMD;
  const ids = new Set<string>(),
    outputs = new Set<string>();
  let relayed = false;
  for (const app of apps) {
    if (
      !app.id ||
      !/^[-a-z0-9]{1,40}$/.test(app.id) ||
      ids.has(app.id)
    )
      throw new Error('empty, duplicate or reserved system app id');
    ids.add(app.id);
    if (composeForm && (!app.input || !isAbsolute(app.input.directory ?? '') || !app.input.files?.length))
      throw new Error('system app requires absolute directory and Compose files');
    if (app.graph && !isAbsolute(app.graph)) throw new Error('system app graph must be absolute');
    if (app.liveRelay) {
      if (relayed) throw new Error('only one system app may set live_relay');
      relayed = true;
      if ((composeForm && !app.configMount?.service) || !isAbsolute(app.configMount?.target ?? '') || app.graph || app.launchProfiles?.length || app.liveOutputs?.length)
        throw new Error('live_relay requires a config mount and excludes graph, launch profiles and live outputs');
      if ((app.liveRelay.port ?? 0) > 65535) throw new Error('live_relay port must be in 0..65535');
    }
    for (const mount of app.outputMounts ?? [])
      if ((composeForm && !mount.service) || !isAbsolute(mount.target ?? ''))
        throw new Error('output mount requires a service and absolute target');
    validateOutputs(app.recordings ?? [], app.liveOutputs ?? [], outputs);
    const profiles = app.launchProfiles ?? [];
    if (profiles.length && ((composeForm && !app.configMount?.service) || !isAbsolute(app.configMount?.target ?? '')))
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
export function appOptions(apps: App[], selected: SelectedApp[], required: string[] = []) {
  return [

    ...apps.map((app) => {
      const chosen = selected.find((item) => item.id === app.id),
        options = app.launchProfiles ?? [];
      return {
        id: app.id,
        label: app.label,
        selected: !!chosen || required.includes(app.id!),
        required: required.includes(app.id!),
        profiles: options.map(({ id, label }) => ({ id, label })),
        profile: chosen?.profile?.id ?? options[0]?.id ?? '',
      };
    }),
  ];
}
export function outputs(apps: App[]): Output[] {
  return apps.flatMap((app) => app.recordings ?? []);
}
/** Resolve selected config and output mounts; each backend supplies the app's output root. */
export function appBindings(app: SelectedApp, artifacts: string, relaySettings?: string) {
  const target = app.configMount?.target;
  const config = app.profile?.config ?? (app.liveRelay ? relaySettings : undefined);
  return {
    config: target && config ? { source: config, target, readOnly: true, service: app.configMount?.service } : undefined,
    outputs: (app.outputMounts ?? []).map((mount) => ({
      source: artifacts, target: mount.target!, readOnly: false, service: mount.service,
    })),
  };
}
