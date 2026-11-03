import { execFile } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { chmod } from 'node:fs/promises';
import { basename, dirname, join } from 'node:path';
import { promisify } from 'node:util';
import { writeOnce } from './files.js';
import { serviceImages } from './images.js';
import type { SelectedApp } from './system-apps.js';
const exec = promisify(execFile);
const RESOLVE_TIMEOUT_MS = 30_000;
const RESOLVED_BYTES = 1024 * 1024;

/** The resolved launch config of `id` inside a review workspace. */
export const resolvedConfig = (workspace: string, id: string) => join(workspace, `${id}.resolved.pbtxt`);

/**
 * Run each selected `config_resolver` in a sandboxed one-shot container, writing its stdout
 * to `resolvedConfig`. Images must be local. Errors name the app and carry its stderr on a
 * failure, timeout or empty output; `Stopped` after `signal` aborts. Removes the container.
 */
export async function resolveConfigs(socket: string, apps: SelectedApp[], workspace: string, signal: AbortSignal, timeout = RESOLVE_TIMEOUT_MS) {
  const docker = ['--host', `unix://${socket}`];
  for (const app of apps) {
    if (!app.configResolver || !app.profile) continue;
    const image = (await serviceImages(socket, app.input!))[app.configMount!.service!];
    if (!image) throw new Error(`${app.id}: config_mount service has no image`);
    const [command, ...args] = app.configResolver.command!,
      name = `doramcap-resolve-${randomBytes(8).toString('hex')}`;
    try {
      const { stdout } = await exec('docker', [
        ...docker, 'run', '--rm', '--name', name, '--pull', 'never',
        '--network', 'none', '--read-only', '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges',
        '--mount', `type=bind,source=${dirname(app.profile.config!)},target=/config,readonly`,
        '--entrypoint', command, image, ...args, `/config/${basename(app.profile.config!)}`,
      ], { signal, timeout, killSignal: 'SIGKILL', maxBuffer: RESOLVED_BYTES });
      if (!stdout.trim()) throw new Error('empty output');
      const path = resolvedConfig(workspace, app.id!);
      await writeOnce(path, Buffer.from(stdout));
      // The frozen copy keeps this mode, and the app reads it as its own user.
      await chmod(path, 0o644);
    } catch (error) {
      // A killed client leaves its container running.
      await exec('docker', [...docker, 'rm', '--force', name], { timeout: 30_000 }).catch(() => {});
      if (signal.aborted) throw new Error('Stopped');
      const { stderr, message } = error as Error & { stderr?: string };
      throw new Error(`${app.id}: config resolver failed: ${stderr?.trim() || message}`);
    }
  }
}
