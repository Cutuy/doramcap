// Run the existing standalone Node suites in both CI environments.
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const suites = [
  ...readdirSync(join(root, 'apps')).sort().map(app => `apps/${app}/frontend`),
  'console/server', 'console/frontend',
];
const run = (command: string, args: string[], cwd: string) => {
  const result = spawnSync(command, args, { cwd, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
};
for (const suite of suites) {
  const directory = join(root, suite);
  const manifest = join(directory, 'package.json');
  if (!existsSync(manifest)) continue;
  const { scripts } = JSON.parse(readFileSync(manifest, 'utf8'));
  if (!scripts?.test) continue;
  console.log(`\nNode suite: ${suite}`);
  run('npm', ['ci', '--no-audit', '--no-fund'], directory);
  run('npm', ['test'], directory);
  if (suite.startsWith('console/')) run('npm', ['run', 'build'], directory);
}
if (existsSync(join(root, 'console/contract/browser.test.ts'))) {
  console.log('\nConsole browser/server contract');
  run('npm', ['exec', '--', 'tsx', '--test', '../contract/browser.test.ts'], join(root, 'console/server'));
}
