import { readFileSync, writeFileSync } from 'node:fs';
import { parseArgs } from 'node:util';

const { values } = parseArgs({ options: {
  'base-url': { type: 'string' }, version: { type: 'string' },
  'amd64-sha256': { type: 'string' }, 'arm64-sha256': { type: 'string' }, output: { type: 'string' },
} });
for (const key of ['base-url', 'version', 'amd64-sha256', 'arm64-sha256', 'output'] as const)
  if (!values[key]) throw new Error(`Required: --${key}`);
const base = new URL(values['base-url']!);
if (base.protocol !== 'https:' || base.username || base.password || base.search || base.hash ||
    !/^[a-zA-Z0-9:/._%-]+$/.test(base.href)) throw new Error('Use an HTTPS release directory URL');
if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,79}$/.test(values.version!)) throw new Error('Invalid version');
for (const key of ['amd64-sha256', 'arm64-sha256'] as const)
  if (!/^[0-9a-f]{64}$/.test(values[key]!)) throw new Error(`Invalid --${key}`);
const script = readFileSync(new URL('install.template.sh', import.meta.url), 'utf8');
writeFileSync(values.output!, script.replaceAll('@@AMD64@@', values['amd64-sha256']!)
  .replaceAll('@@ARM64@@', values['arm64-sha256']!).replaceAll('@@VERSION@@', values.version!)
  .replaceAll('@@BASE@@', base.href.replace(/\/$/, '')), { mode: 0o755 });
