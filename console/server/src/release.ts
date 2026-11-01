import { cp, lstat, mkdir, readFile, readlink, realpath, rename, rm, symlink, writeFile } from 'node:fs/promises';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { capabilities, wire, type Config } from './native.js';

export async function readRelease(root: string) {
  const manifest = wire.ReleaseManifest.decode(await readFile(join(root, 'release.pb')));
  if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,79}$/.test(manifest.version)) throw new Error('Invalid release version');
  if (process.platform !== 'linux' || manifest.platform !== `linux-${process.arch}`)
    throw new Error(`Release ${manifest.platform} does not support ${process.platform}-${process.arch}`);
  if (!manifest.config) throw new Error('Release configuration is missing');
  return manifest;
}

/** Resolve release-owned paths inside a writable playground; preserve operator configuration on reuse. */
export function releaseConfig(template: Config, playground: string, state: string): Config {
  const config = wire.ConsoleConfig.decode(wire.ConsoleConfig.encode(template).finish());
  const input = (path: string, base = playground) => {
    if (isAbsolute(path)) throw new Error(`Release input must be relative to playground: ${path}`);
    const target = resolve(base, path), inside = relative(playground, target);
    if (inside === '..' || inside.startsWith(`..${sep}`)) throw new Error('Release input escapes playground');
    return target;
  };
  config.projectRoot = input(config.projectRoot || '.');
  config.dataRoot = join(state, 'runs');
  config.dockerSocket ||= '/var/run/docker.sock';
  config.listenAddress = '127.0.0.1:43187';
  config.publicOrigin = 'http://127.0.0.1:43187';
  config.accessSecretFile = '';
  config.downloadMissingImages = true;
  if (config.dora?.graph) config.dora.graph = input(config.dora.graph);
  if (config.dora) {
    config.dora.runtimeDirectory = '';
    config.dora.zenohOverlay = '';
    config.dora.graphSha256 = '';
  }
  for (const app of config.systemApps) {
    if (!app.input) throw new Error(`Missing Compose input: ${app.id}`);
    app.input.directory = input(app.input.directory || '.');
    for (const file of [...(app.input.files ?? []), ...(app.input.envFiles ?? [])]) input(file, app.input.directory);
    if (app.graph) app.graph = input(app.graph);
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
  const config = releaseConfig(wire.ConsoleConfig.create(release.config!), playground, state);
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
    await cp(source, staging, { recursive: true, errorOnExist: true, force: false });
    await rename(staging, target);
  } catch (error) { await rm(staging, { recursive: true, force: true }); throw error; }
  const next = join(prefix, `.current-${process.pid}`);
  await symlink(relative(prefix, target), next);
  await rename(next, join(prefix, 'current'));
  try { await symlink(join(prefix, 'current/bin/doramcap-console'), bin); }
  catch (error) { if ((error as NodeJS.ErrnoException).code !== 'EEXIST') throw error; }
  return target;
}
