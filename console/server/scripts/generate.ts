import { execFileSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';

mkdirSync('src/generated', { recursive: true });
execFileSync('node_modules/.bin/pbjs', [
  '-t', 'static-module', '-w', 'commonjs',
  '-p', '../proto', '-p', '../../libs/workload-docker/proto',
  '-p', '../../libs/telemetry/workflow-protocol/proto',
  '-p', '../../libs/telemetry/protocol/proto',
  '-o', 'src/generated/console.cjs', '../proto/console.proto',
], { stdio: 'inherit' });
execFileSync('node_modules/.bin/pbts', [
  '-o', 'src/generated/console.d.cts', 'src/generated/console.cjs',
], { stdio: 'inherit' });
