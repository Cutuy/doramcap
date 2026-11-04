// Publish release images and write a release template pinned to their multi-arch digests.
//   build: build and push this architecture's images from target/release and --dora.
//   merge: join both architectures and write the pinned template to --output.
import { execFileSync } from 'node:child_process';
import { cpSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const { values, positionals: [command] } = parseArgs({ allowPositionals: true, options: {
  registry: { type: 'string' }, version: { type: 'string' }, source: { type: 'string' },
  arch: { type: 'string' }, dora: { type: 'string' }, output: { type: 'string' },
} });
const { registry, version, source = 'https://github.com/Cutuy/doramcap' } = values;
if (!registry || !version) throw new Error('usage: build|merge --registry REGISTRY --version VERSION [--arch ARCH --dora PATH] [--output DIR]');
const images: Record<string, { dockerfile: string; binary?: string }> = {
  dora: { dockerfile: 'libs/workload-docker/deploy/Dockerfile', binary: values.dora ?? '' },
  'system-monitor': { dockerfile: 'apps/system-monitor/deploy/Dockerfile', binary: 'target/release/system-monitor' },
  'mcap-recorder': { dockerfile: 'apps/mcap-recorder/deploy/Dockerfile', binary: 'target/release/mcap-recorder' },
  'live-bridge': { dockerfile: 'console/bridge/Dockerfile', binary: 'target/release/live-bridge' },
  'cpu-burn': { dockerfile: 'console/release/playground/cpu-burn/Dockerfile' },
};
const docker = (...args: string[]) => execFileSync('docker', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'] });
const digest = (reference: string) => docker('buildx', 'imagetools', 'inspect', reference, '--format', '{{json .Manifest}}');

if (command === 'build') {
  if (!values.arch || !values.dora) throw new Error('build requires --arch and --dora');
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
  cpSync(join(root, 'libs/telemetry/protocol/proto/telemetry_descriptor.bin'), join(template, 'system-apps/data-recorder/telemetry_descriptor.bin'));
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
} else throw new Error('usage: build|merge');
