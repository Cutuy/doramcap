import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, realpathSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const { values } = parseArgs({ options: {
  output: { type: 'string' }, version: { type: 'string' }, node: { type: 'string' },
  playground: { type: 'string' }, 'system-apps': { type: 'string' }, 'node-license': { type: 'string' }, config: { type: 'string' },
} });
for (const key of ['output', 'version', 'node', 'node-license', 'playground', 'system-apps', 'config'] as const)
  if (!values[key]) throw new Error(`Required: --${key}`);
if (process.platform !== 'linux' || !['x64', 'arm64'].includes(process.arch)) throw new Error('Build on Linux x64 or arm64');
if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,79}$/.test(values.version!)) throw new Error('Invalid version');
const output = resolve(values.output!);
if (existsSync(output)) throw new Error(`Output already exists: ${output}`);
const node = resolve(values.node!);
const pinnedNode = readFileSync(join(root, '.node-version'), 'utf8').trim();
if (execFileSync(node, ['-p', 'process.versions.node+" "+process.platform+"-"+process.arch'], { encoding: 'utf8' }).trim()
  !== `${pinnedNode} linux-${process.arch}`) throw new Error('Node must match .node-version and the build platform');
const playground = resolve(values.playground!);
const systemApps = resolve(values['system-apps']!);
const projectDirectories = new Set<string>();
function validateTree(path: string) {
  const info = lstatSync(path);
  if (info.isDirectory()) {
    for (const name of readdirSync(path)) {
      if (['.git', '.env', 'node_modules', '.sudo-password'].includes(name) || /\.(pem|key)$/.test(name))
        throw new Error(`Remove private/generated release input: ${join(path, name)}`);
      if (['compose.yaml', 'compose.yml', 'docker-compose.yaml', 'docker-compose.yml'].includes(name)) projectDirectories.add(path);
      validateTree(join(path, name));
    }
  } else if (!info.isFile()) throw new Error(`Release inputs must be regular files: ${path}`);
}
validateTree(systemApps);
// Only playground directories are user projects; configuration lists the system apps.
projectDirectories.clear();
validateTree(playground);
const run = (command: string, args: string[], cwd = root) => execFileSync(command, args, { cwd, stdio: 'inherit' });
run('npm', ['ci', '--no-audit', '--no-fund'], `${root}/console/server`);
run('cargo', ['build', '--release', '--locked', '-p', 'console-native']);
run(node, ['scripts/copy-native.ts', 'release'], `${root}/console/server`);
run('npm', ['run', 'build'], `${root}/console/server`);
run('npm', ['ci', '--no-audit', '--no-fund'], `${root}/console/frontend`);
run('npm', ['run', 'build'], `${root}/console/frontend`);
const { capabilities, wire } = await import(pathToFileURL(join(root, 'console/server/dist/native.js')).href);
const config = capabilities.parseConfig(readFileSync(resolve(values.config!), 'utf8'));
const { releaseConfig } = await import(pathToFileURL(join(root, 'console/server/dist/release.js')).href);
const deployed = releaseConfig(config, playground, systemApps, '/state');
const { discover } = await import(pathToFileURL(join(root, 'console/server/dist/system-apps.js')).href);
const { imageReferences, requireMultiArch } = await import(pathToFileURL(join(root, 'console/server/dist/images.js')).href);
const projects = await Promise.all([...projectDirectories].map(path => discover(path, ['*'])));
projects.push(...deployed.systemApps.map((app: { input: unknown }) => app.input));
if (!projects.length) throw new Error('Playground must contain a Compose project');
const images = new Set<string>([config.dora?.image ?? '']);
for (const project of projects) for (const image of await imageReferences(deployed.dockerSocket, project)) images.add(image);
for (const image of images) {
  if (!/@sha256:[0-9a-f]{64}$/.test(image)) throw new Error(`Pin release image by digest: ${image || 'Dora'}`);
  // Anonymous, so this also proves users can pull the image.
  requireMultiArch(image, execFileSync('docker', ['buildx', 'imagetools', 'inspect', '--raw', image], { encoding: 'utf8' }));
}
const packageRoot = join(output, 'package');
mkdirSync(join(packageRoot, 'server'), { recursive: true });
cpSync(`${root}/console/server/dist`, `${packageRoot}/server/dist`, { recursive: true });
cpSync(`${root}/console/server/build`, `${packageRoot}/server/build`, { recursive: true });
for (const name of ['package.json', 'package-lock.json']) cpSync(`${root}/console/server/${name}`, `${packageRoot}/server/${name}`);
run('npm', ['ci', '--omit=dev', '--ignore-scripts', '--no-audit', '--no-fund'], `${packageRoot}/server`);
cpSync(`${root}/console/frontend/dist`, `${packageRoot}/frontend`, { recursive: true });
cpSync(playground, `${packageRoot}/playground`, { recursive: true });
cpSync(systemApps, `${packageRoot}/system-apps`, { recursive: true });
mkdirSync(`${packageRoot}/runtime`);
// Copy the real binary: a symlink would dangle on every other host.
cpSync(realpathSync(node), `${packageRoot}/runtime/node`);
cpSync(realpathSync(values['node-license']!), `${packageRoot}/runtime/NODE-LICENSE`, { force: false });
mkdirSync(`${packageRoot}/bin`);
writeFileSync(`${packageRoot}/bin/doramcap-console`, `#!/bin/sh\nset -eu\nroot=$(dirname "$(dirname "$(readlink -f "$0")")")\nexec "$root/runtime/node" "$root/server/dist/cli.js" "$@"\n`, { mode: 0o755 });
writeFileSync(`${packageRoot}/release.pb`, wire.ReleaseManifest.encode({
  version: values.version, platform: `linux-${process.arch}`, config,
}).finish());
cpSync(`${root}/console/scripts/release-readme.md`, `${packageRoot}/README.md`);
const arch = process.arch === 'x64' ? 'amd64' : 'arm64';
const filename = `doramcap-console-${values.version}-linux-${arch}.tar.gz`;
run('tar', ['-czf', join(output, filename), '-C', output, 'package']);
const sha256 = createHash('sha256').update(readFileSync(join(output, filename))).digest('hex');
writeFileSync(join(output, `${filename}.sha256`), `${sha256}  ${filename}\n`);
console.log(`Release archive: ${join(output, filename)}\nSHA256: ${sha256}`);
