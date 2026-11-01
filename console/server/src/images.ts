import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import type { Config, Project } from './native.js';
const exec = promisify(execFile);

/** Resolve the selected Compose services without downloading images or starting containers. */
export async function imageReferences(socket: string, project: Project): Promise<string[]> {
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
  return Object.entries(model.services ?? {}).map(([name, service]) => {
    if (typeof service.image !== 'string' || !service.image) throw new Error(`${name}: prebuilt image required`);
    return service.image;
  });
}

/** Pull missing images before local image inspection; successful pulls remain in Docker's cache. */
export async function ensureImages(config: Config, projects: Project[]): Promise<void> {
  const images = new Set<string>([config.dora?.image || 'doramcap/dora:local']);
  for (const project of projects)
    for (const image of await imageReferences(config.dockerSocket, project)) images.add(image);
  for (const image of images) {
    const args = ['--host', `unix://${config.dockerSocket}`];
    try { await exec('docker', [...args, 'image', 'inspect', image], { timeout: 30_000 }); }
    catch {
      console.log(`Downloading image: ${image}`);
      await new Promise<void>((resolve, reject) => {
        const child = spawn('docker', [...args, 'pull', image], { stdio: ['ignore', 'inherit', 'inherit'] });
        child.once('error', reject);
        child.once('exit', code => code === 0 ? resolve() : reject(new Error(`Image download failed: ${image}`)));
      });
    }
  }
}
