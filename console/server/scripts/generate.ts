import { execFileSync } from 'node:child_process';
import { mkdirSync, cpSync } from 'node:fs';

mkdirSync('src/generated', { recursive: true });
cpSync('../contract/transport.ts', 'src/generated/transport.ts');
execFileSync('node_modules/.bin/pbjs', [
  '-t', 'static-module', '-w', 'commonjs',
  '-p', '../proto', '-p', '../../libs/workload-docker/proto',
  '-p', '../../libs/telemetry/workflow-protocol/proto',
  '-p', '../../libs/telemetry/protocol/proto',
  '-o', 'src/generated/console.cjs', '../proto/console.proto', '../proto/execution.proto', '../../apps/cpu-viewer/server/proto/delivery.proto',
], { stdio: 'inherit' });
execFileSync('node_modules/.bin/pbts', [
  '-o', 'src/generated/console.d.cts', 'src/generated/console.cjs',
], { stdio: 'inherit' });
