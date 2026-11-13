// Publish release images and write a release template pinned to their multi-arch digests.
//   build: build and push this architecture's images from target/release and --dora.
//   merge: join both architectures and write the pinned template to --output.
//   promote: point latest at the verified, published multi-arch version.
import { execFileSync } from 'node:child_process';
import { cpSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { stageSystemAppInputs } from './system-app-inputs.mts';
import { resolvedDoraVersion, verifyDoraBinary } from '../../scripts/ci/dora-release.mts';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const { values, positionals: [command] } = parseArgs({ allowPositionals: true, options: {
  registry: { type: 'string' }, version: { type: 'string' }, source: { type: 'string' },
  arch: { type: 'string' }, dora: { type: 'string' }, output: { type: 'string' },
} });
const { registry, version, source = 'https://github.com/Cutuy/doramcap' } = values;
if (!registry || !version) throw new Error('usage: build|merge|promote --registry REGISTRY --version VERSION [--arch ARCH --dora PATH] [--output DIR]');
const images: Record<string, { dockerfile: string; binary?: string }> = {
  dora: { dockerfile: 'libs/workload-docker/deploy/Dockerfile', binary: values.dora ?? '' },
  'system-monitor': { dockerfile: 'apps/system-monitor/deploy/Dockerfile', binary: 'target/release/system-monitor' },
  'mcap-recorder': { dockerfile: 'apps/mcap-recorder/deploy/Dockerfile', binary: 'target/release/mcap-recorder' },
  'live-bridge': { dockerfile: 'console/bridge/Dockerfile', binary: 'target/release/live-bridge' },
  'fault-watchdog': { dockerfile: 'apps/fault-watchdog/deploy/standalone/Dockerfile', binary: 'target/release/fault-watchdog' },
  'cpu-burn': { dockerfile: 'console/release/playground/cpu-burn/Dockerfile' },
};
const docker = (...args: string[]) => execFileSync('docker', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'] });
const digest = (reference: string) => docker('buildx', 'imagetools', 'inspect', reference, '--format', '{{json .Manifest}}');

if (command === 'build') {
  if (!values.arch || !values.dora) throw new Error('build requires --arch and --dora');
  verifyDoraBinary(values.dora, resolvedDoraVersion(root));
  for (const [name, { dockerfile, binary }] of Object.entries(images)) {
    const stage = mkdtempSync(join(tmpdir(), 'console-image-')), tag = `${registry}/${name}:${version}-${values.arch}`;
    if (binary) cpSync(resolve(root, binary), join(stage, binary.split('/').at(-1)!));
    // The source label links each package to the public repository.
    docker('build', '--tag', tag, '--label', `org.opencontainers.image.source=${source}`, '--file', dockerfile, stage);
    docker('push', tag);
    rmSync(stage, { recursive: true });
  }
} else if (command === 'merge') {
  if (!values.output) throw new Error('merge requires --output');
  const template = resolve(values.output);
  cpSync(join(root, 'console/release'), template, { recursive: true });
  stageSystemAppInputs(root, join(template, 'system-apps'));
  const index = (name: string, sources: string[]) => {
    const tag = `${registry}/${name}:${version}`;
    docker('buildx', 'imagetools', 'create', '--tag', tag, ...sources);
    const pinned = JSON.parse(digest(tag)) as { digest: string };
    return `${registry}/${name}@${pinned.digest}`;
  };
  const replacements = new Map(Object.keys(images).map(name => [`doramcap/${name}:local`,
    index(name, ['amd64', 'arm64'].map(arch => `${registry}/${name}:${version}-${arch}`))]));
  const rewrite = (path: string) => {
    if (statSync(path).isDirectory()) return readdirSync(path).forEach(name => rewrite(join(path, name)));
    const text = readFileSync(path, 'utf8');
    let next = text;
    for (const [from, to] of replacements) next = next.replaceAll(from, to);
    if (next !== text) writeFileSync(path, next);
  };
  rewrite(template);
} else if (command === 'promote') {
  // Check every source before changing any moving tag; publish the captured digests.
  const published = Object.keys(images).map(name => {
    const tag = `${registry}/${name}:${version}`;
    const pinned = JSON.parse(digest(tag)) as { digest: string };
    if (!/^sha256:[0-9a-f]{64}$/.test(pinned.digest)) throw new Error(`Invalid image digest: ${tag}`);
    const source = `${registry}/${name}@${pinned.digest}`;
    const manifest = JSON.parse(docker('buildx', 'imagetools', 'inspect', source, '--raw')) as {
      manifests?: { platform?: { os?: string; architecture?: string } }[];
    };
    for (const architecture of ['amd64', 'arm64'])
      if (!manifest.manifests?.some(entry => entry.platform?.os === 'linux' && entry.platform.architecture === architecture))
        throw new Error(`Published image lacks linux/${architecture}: ${tag}`);
    return { latest: `${registry}/${name}:latest`, source, digest: pinned.digest };
  });
  for (const image of published) {
    docker('buildx', 'imagetools', 'create', '--tag', image.latest, image.source);
    const promoted = JSON.parse(digest(image.latest)) as { digest: string };
    if (promoted.digest !== image.digest) throw new Error(`Promoted digest differs: ${image.latest}`);
    console.log(`${image.latest} -> ${image.digest}`);
  }
} else throw new Error('usage: build|merge|promote');
