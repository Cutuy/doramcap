// Run the existing standalone Node suites in both CI environments.
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
for (const app of readdirSync(join(root, 'apps')).sort()) {
  const frontend = join(root, 'apps', app, 'frontend');
  const manifest = join(frontend, 'package.json');
  if (!existsSync(manifest)) continue;
  const { scripts } = JSON.parse(readFileSync(manifest, 'utf8'));
  if (!scripts?.test) continue;
  console.log(`\nNode suite: ${app}`);
  for (const args of [['ci', '--no-audit', '--no-fund'], ['test']]) {
    const result = spawnSync('npm', args, {
      cwd: frontend,
      stdio: 'inherit',
    });
    if (result.error) throw result.error;
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}
