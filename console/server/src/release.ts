import { execFile } from 'node:child_process';
import { cp, lstat, mkdir, readFile, readlink, realpath, rename, rm, statfs, symlink, writeFile } from 'node:fs/promises';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { promisify } from 'node:util';
import { capabilities, wire, type Config } from './native.js';

const exec = promisify(execFile);
const MIN_DOCKER_FREE_BYTES = 10 * 1024 ** 3;

export async function readRelease(root: string) {
  const manifest = wire.ReleaseManifest.decode(await readFile(join(root, 'release.pb')));
  if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,79}$/.test(manifest.version)) throw new Error('Invalid release version');
  if (process.platform !== 'linux' || manifest.platform !== `linux-${process.arch}`)
    throw new Error(`Release ${manifest.platform} does not support ${process.platform}-${process.arch}`);
  if (!manifest.config) throw new Error('Release configuration is missing');
  return manifest;
}

/** Resolve project paths inside a writable playground and system app paths inside the immutable release. */
export function releaseConfig(template: Config, playground: string, systemApps: string, state: string): Config {
  const config = wire.ConsoleConfig.decode(wire.ConsoleConfig.encode(template).finish());
  const input = (root: string, path: string, base = root) => {
    if (isAbsolute(path)) throw new Error(`Release input must be relative: ${path}`);
    const target = resolve(base, path), inside = relative(root, target);
    if (inside === '..' || inside.startsWith(`..${sep}`)) throw new Error(`Release input escapes ${root}: ${path}`);
    return target;
  };
  config.projectRoot = input(playground, config.projectRoot || '.');
  config.dataRoot = join(state, 'runs');
  config.dockerSocket ||= '/var/run/docker.sock';
  config.listenAddress = '127.0.0.1:43187';
  config.publicOrigin = 'http://127.0.0.1:43187';
  config.accessSecretFile = '';
  config.downloadMissingImages = true;
  if (config.dora?.graph) config.dora.graph = input(playground, config.dora.graph);
  if (config.dora) {
    config.dora.runtimeDirectory = '';
    config.dora.zenohOverlay = '';
    config.dora.graphSha256 = '';
  }
  for (const app of config.systemApps) {
    if (!app.input) throw new Error(`Missing Compose input: ${app.id}`);
    app.input.directory = input(systemApps, app.input.directory || '.');
    for (const file of [...(app.input.files ?? []), ...(app.input.envFiles ?? [])]) input(systemApps, file, app.input.directory);
    if (app.graph) app.graph = input(systemApps, app.graph);
    for (const profile of app.launchProfiles ?? []) profile.config = input(systemApps, profile.config ?? '');
  }
  return config;
}

/** Require a canonical root-owned directory before writing privileged state. */
export async function privateState(path: string) {
  if (!isAbsolute(path)) throw new Error('State directory must be absolute');
  await mkdir(path, { recursive: true, mode: 0o700 });
  const info = await lstat(path);
  if (!info.isDirectory() || info.uid !== 0 || (info.mode & 0o077) !== 0 || await realpath(path) !== path)
    throw new Error('State directory must be canonical, root-owned and mode 0700');
}

export async function initialize(root: string, state: string) {
  const release = await readRelease(root);
  await privateState(state);
  const configPath = join(state, 'console.pbtxt');
  try {
    const info = await lstat(configPath);
    if (!info.isFile() || info.uid !== 0 || (info.mode & 0o022)) throw new Error('Unsafe Console configuration');
    return configPath;
  } catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error; }
  const playground = join(state, 'playground');
  // Seed once; later releases keep all user-edited playground files.
  try { await lstat(playground); }
  catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    const staging = join(state, `playground-${process.pid}`);
    await cp(join(root, 'playground'), staging, { recursive: true, errorOnExist: true, force: false });
    await rename(staging, playground);
  }
  // The config stays coupled to the release that wrote it, through that release's own directory.
  const config = releaseConfig(wire.ConsoleConfig.create(release.config!), playground, join(root, 'system-apps'), state);
  await writeFile(configPath, capabilities.formatConfig(config), { mode: 0o600, flag: 'wx' });
  return configPath;
}

async function installDirectory(path: string) {
  await mkdir(path, { recursive: true, mode: 0o755 });
  const info = await lstat(path);
  if (!info.isDirectory() || info.uid !== process.getuid!() || (info.mode & 0o022) || await realpath(path) !== path)
    throw new Error(`Unsafe installation directory: ${path}`);
}

/** Install an immutable version and atomically switch the current symlink; prior versions stay available. */
export async function installRelease(source: string, prefix = '/opt/doramcap-console', bin = '/usr/local/bin/doramcap-console') {
  const manifest = await readRelease(source);
  try {
    const info = await lstat(bin);
    if (!info.isSymbolicLink() || await readlink(bin) !== join(prefix, 'current/bin/doramcap-console'))
      throw new Error(`Refusing to replace another launcher: ${bin}`);
  } catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error; }
  await installDirectory(prefix);
  await installDirectory(join(prefix, 'releases'));
  await installDirectory(dirname(bin));
  const target = join(prefix, 'releases', manifest.version);
  try { await lstat(target); throw new Error(`Release already installed: ${target}`); }
  catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error; }
  const staging = join(prefix, 'releases', `.install-${process.pid}`);
  try {
    await cp(source, staging, { recursive: true, errorOnExist: true, force: false, verbatimSymlinks: true });
    await rename(staging, target);
  } catch (error) { await rm(staging, { recursive: true, force: true }); throw error; }
  const next = join(prefix, `.current-${process.pid}`);
  await symlink(relative(prefix, target), next);
  await rename(next, join(prefix, 'current'));
  try { await symlink(join(prefix, 'current/bin/doramcap-console'), bin); }
  catch (error) { if ((error as NodeJS.ErrnoException).code !== 'EEXIST') throw error; }
  return target;
}

/**
 * Report every host requirement with its remediation.
 * Warnings keep Console usable without optional apps; throws when a required check fails.
 */
export async function doctor(socket: string, log: (line: string) => void = console.log) {
  const run = async (program: string, ...args: string[]) =>
    (await exec(program, args, { timeout: 15_000 })).stdout.trim().split('\n')[0];
  const docker = (...args: string[]) => run('docker', '--host', `unix://${socket}`, ...args);
  const checks: { name: string; fix: string; warn?: boolean; probe(): Promise<string> }[] = [
    { name: 'Docker CLI', fix: 'Install Docker Engine: https://docs.docker.com/engine/install/ubuntu/', probe: () => docker('--version') },
    { name: 'Docker Engine', fix: `Start it with: sudo systemctl start docker. Set docker_socket in console.pbtxt if it does not listen on ${socket}.`, async probe() {
      const type = await docker('info', '--format', '{{.OSType}} {{.ServerVersion}}');
      if (!type.startsWith('linux ')) throw new Error(`reports ${type}`);
      return type;
    } },
    { name: 'Docker mount namespace', fix: 'Use Docker Engine from the host package; snap, rootless and Desktop daemons cannot share host OverlayFS mounts.', async probe() {
      const pid = (await run('pidof', 'dockerd')).split(' ')[0];
      if (await readlink(`/proc/${pid}/ns/mnt`) !== await readlink('/proc/self/ns/mnt')) throw new Error('dockerd runs in another mount namespace');
      return 'shared with Console';
    } },
    { name: 'Docker Compose v2', fix: 'Install it with: sudo apt-get install docker-compose-v2', probe: () => docker('compose', 'version') },
    { name: 'rsync', fix: 'Install it with: sudo apt-get install rsync', probe: () => run('rsync', '--version') },
    { name: 'systemd', fix: 'Boot the host with systemd; Console runs as a transient system service.', probe: () => run('systemctl', '--version') },
    { name: 'cgroup v2', fix: 'Boot with systemd.unified_cgroup_hierarchy=1.', async probe() {
      await lstat('/sys/fs/cgroup/cgroup.controllers');
      return 'unified hierarchy';
    } },
    { name: 'OverlayFS', fix: 'Load it with: sudo modprobe overlay', async probe() {
      if (!(await readFile('/proc/filesystems', 'utf8')).includes('overlay')) throw new Error('not in /proc/filesystems');
      return 'available';
    } },
    { name: 'Kernel BTF', warn: true, fix: 'CPU monitoring needs a kernel built with CONFIG_DEBUG_INFO_BTF; other apps are unaffected.', async probe() {
      await lstat('/sys/kernel/btf/vmlinux');
      return 'available';
    } },
    { name: 'Docker disk space', warn: true, fix: 'Free space before downloading images; recordings also grow during runs.', async probe() {
      const { bavail, bsize } = await statfs(await docker('info', '--format', '{{.DockerRootDir}}'));
      const free = bavail * bsize;
      if (free < MIN_DOCKER_FREE_BYTES) throw new Error(`${(free / 1024 ** 3).toFixed(1)} GiB free`);
      return `${(free / 1024 ** 3).toFixed(1)} GiB free`;
    } },
  ];
  let failed = 0;
  for (const { name, fix, warn, probe } of checks) {
    try { log(`ok    ${name}: ${await probe()}`); }
    catch (error) {
      if (!warn) failed++;
      const { code, message } = error as NodeJS.ErrnoException;
      log(`${warn ? 'warn' : 'FAIL'}  ${name}: ${code === 'ENOENT' ? 'not found' : message.split('\n')[0]}`);
      log(`      Fix: ${fix}`);
    }
  }
  if (failed) throw new Error(`${failed} required check${failed === 1 ? '' : 's'} failed; see the fixes above`);
}
