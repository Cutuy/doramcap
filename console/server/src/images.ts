import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import type { Config, Project } from './native.js';
const exec = promisify(execFile);
const PULL_TIMEOUT_MS = 30 * 60_000; // Per image; generous for slow links, bounded for hung registries.

/** Map the selected Compose services to images without downloading images or starting containers. */
export async function serviceImages(socket: string, project: Project): Promise<Record<string, string>> {
  const args = ['--host', `unix://${socket}`, 'compose',
    '--project-directory', project.directory!, '--env-file', '/dev/null'];
  for (const file of project.envFiles ?? []) args.push('--env-file', file);
  for (const file of project.files ?? []) args.push('-f', file);
  for (const profile of project.profiles ?? []) args.push('--profile', profile);
  const { stdout, stderr } = await exec('docker', [...args, 'config', '--format', 'json', ...(project.services ?? [])], {
    cwd: project.directory!, env: { ...process.env, ...project.environment }, timeout: 60_000, maxBuffer: 4 * 1024 * 1024,
  });
  if (stderr.includes('variable is not set')) throw new Error('Unresolved Compose environment');
  const model = JSON.parse(stdout) as { services?: Record<string, { image?: unknown }> };
  return Object.fromEntries(Object.entries(model.services ?? {}).map(([name, service]) => {
    if (typeof service.image !== 'string' || !service.image) throw new Error(`${name}: prebuilt image required`);
    return [name, service.image];
  }));
}

/** Reject a registry manifest that does not offer every architecture a release supports. */
export function requireMultiArch(image: string, raw: string) {
  const manifests = (JSON.parse(raw) as { manifests?: { platform?: { os?: string; architecture?: string } }[] }).manifests ?? [];
  for (const architecture of ['amd64', 'arm64'])
    if (!manifests.some(({ platform }) => platform?.os === 'linux' && platform.architecture === architecture))
      throw new Error(`Pin a multi-arch digest with linux/${architecture}: ${image}`);
}

/**
 * Pull missing images before local image inspection; successful pulls remain in Docker's cache.
 * Errors: `ImageDownload` after a failed or timed-out pull, `Stopped` after `signal` aborts.
 */
export async function ensureImages(config: Config, projects: Project[], signal: AbortSignal, timeout = PULL_TIMEOUT_MS): Promise<void> {
  const images = new Set<string>([config.dora?.image || 'doramcap/dora:local']);
  for (const project of projects)
    for (const image of Object.values(await serviceImages(config.dockerSocket, project))) images.add(image);
  for (const image of images) {
    if (signal.aborted) throw new Error('Stopped');
    const args = ['--host', `unix://${config.dockerSocket}`];
    try { await exec('docker', [...args, 'image', 'inspect', image], { timeout: 30_000, signal }); continue; }
    catch { if (signal.aborted) throw new Error('Stopped'); }
    console.log(`Downloading image: ${image}`);
    try {
      await new Promise<void>((resolve, reject) => {
        // SIGKILL: a wedged client must not outlive the deadline; the daemon cancels its pull.
        const child = spawn('docker', [...args, 'pull', image],
          { stdio: ['ignore', 'inherit', 'inherit'], signal, timeout, killSignal: 'SIGKILL' });
        child.once('error', reject);
        child.once('exit', (code, killed) => code === 0 ? resolve() : reject(new Error(killed ? `killed after ${timeout} ms` : `exit ${code}`)));
      });
    } catch (error) {
      if (signal.aborted) throw new Error('Stopped');
      console.error(`Image download failed: ${image}: ${(error as Error).message}`);
      throw new Error('ImageDownload');
    }
  }
}
