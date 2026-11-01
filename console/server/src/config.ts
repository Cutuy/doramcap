import { randomBytes } from 'node:crypto';
import { readFile, realpath } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { isIP } from 'node:net';
import { capabilities, wire, type Config } from './native.js';
import { validateApps } from './system-apps.js';

/** Decode pbtxt with the existing parser and validate server-owned deployment policy. */
export async function loadConfig(path: string): Promise<{ config: Config; secret: string }> {
  const config = capabilities.parseConfig(await readFile(path, 'utf8'));
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
  for (const [name, value] of [
    ['project_root', config.projectRoot],
    ['data_root', config.dataRoot],
    ['docker_socket', config.dockerSocket],
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
  if (config.liveBridge && (!config.liveBridge.image || !config.liveBridge.port || config.liveBridge.port > 65535))
    throw new Error('live_bridge requires an image and a port in 1..65535');
  validateApps(config.systemApps);
}
