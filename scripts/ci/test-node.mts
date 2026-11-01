// Run the existing standalone Node suites in both CI environments.
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
for (const app of [
  'activity-demo',
  'cpu-viewer',
  'cuda-shm-demo',
  'dataflow-tracing',
  'orchestrator-demo',
  'storage-demo',
]) {
  console.log(`\nNode suite: ${app}`);
  for (const args of [['ci', '--no-audit', '--no-fund'], ['test']]) {
    const result = spawnSync('npm', args, {
      cwd: join(root, 'apps', app, 'frontend'),
      stdio: 'inherit',
    });
    if (result.error) throw result.error;
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}
