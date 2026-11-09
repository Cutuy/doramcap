import { randomBytes } from 'node:crypto';
import { readFile, realpath } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { isIP } from 'node:net';
import { capabilities, launch, wire, type Config } from './native.js';
import { validateApps } from './system-apps.js';
import { expandManifests } from './manifests.js';

/** Decode pbtxt with the existing parser and validate server-owned deployment policy. */
export async function loadConfig(path: string): Promise<{ config: Config; secret: string }> {
  const config = capabilities.parseConfig(await readFile(path, 'utf8'));
  expandManifests(config);
  validateConfig(config);
  const secret = (
    config.accessSecretFile ? await readFile(config.accessSecretFile, 'utf8').catch((error) => {
      throw new Error(`access_secret_file: ${error}`);
    }) : randomBytes(32).toString('base64url')
  ).replace(/\r?\n$/, '');
  if (!secret) throw new Error('access_secret_file must contain a nonempty secret');
  config.projectRoot = await realpath(config.projectRoot);
  return { config, secret };
}
export function validateConfig(config: Config) {
  if (![launch.HostShmAccess.PASSTHROUGH, launch.HostShmAccess.BLOCKED].includes(config.hostShmAccess))
    throw new Error('host_shm_access must be PASSTHROUGH or BLOCKED');
  switch (config.form ?? launch.Form.FORM_UNSPECIFIED) {
    case launch.Form.FORM_UNSPECIFIED:
    case launch.Form.COMPOSE:
      if (!isAbsolute(config.dockerSocket)) throw new Error('docker_socket must be absolute');
      break;
    case launch.Form.SYSTEMD:
      if (config.dockerSocket) throw new Error('docker_socket applies only to COMPOSE');
      break;
    default:
      throw new Error('form: only COMPOSE and SYSTEMD are supported');
  }
  for (const [name, value] of [
    ['project_root', config.projectRoot],
    ['data_root', config.dataRoot],
  ])
    if (!isAbsolute(value)) throw new Error(`${name} must be absolute`);
  if (config.accessSecretFile && !isAbsolute(config.accessSecretFile))
    throw new Error('access_secret_file must be absolute');
  const origin = new URL(config.publicOrigin);
  if (
    !['http:', 'https:'].includes(origin.protocol) ||
    origin.origin !== config.publicOrigin ||
    origin.username ||
    origin.password
  )
    throw new Error('public_origin must be a canonical HTTP(S) origin');
  const address = /^(\[[^\]]+\]|[^:]+):(\d+)$/.exec(config.listenAddress);
  if (
    !address ||
    !isIP(address[1].replace(/^\[|\]$/g, '')) ||
    Number(address[2]) > 65535 ||
    Number(address[2]) === 0
  )
    throw new Error('invalid listen_address');
  const runtime = config.runtime ?? wire.RuntimeConfig.create();
  runtime.readinessSeconds ??= 60;
  runtime.stopSeconds ??= 15;
  runtime.sampleIntervalMs ??= 100;
  if (runtime.readinessSeconds < 1) throw new Error('runtime.readiness_seconds must be positive');
  if (runtime.stopSeconds < 1 || runtime.stopSeconds > 300)
    throw new Error('runtime.stop_seconds must be in 1..300');
  if (runtime.sampleIntervalMs < 1) throw new Error('runtime.sample_interval_ms must be positive');
  config.runtime = runtime;
  validateApps(config.apps, config.form);
  if (new Set(config.doraApps).size !== config.doraApps.length || config.doraApps.some((id) => !config.apps.some((app) => app.id === id)))
    throw new Error('dora_apps must name unique configured apps');
}
