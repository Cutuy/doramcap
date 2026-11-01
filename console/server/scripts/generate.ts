import { execFileSync } from 'node:child_process';
import { mkdirSync, cpSync, readFileSync, writeFileSync } from 'node:fs';

mkdirSync('src/generated', { recursive: true });
cpSync('../contract/transport.ts', 'src/generated/transport.ts');
execFileSync('node_modules/.bin/pbjs', [
  '-t', 'static-module', '-w', 'es6',
  '-p', '../proto', '-p', '../../libs/workload-docker/proto',
  '-p', '../../libs/telemetry/workflow-protocol/proto',
  '-p', '../../libs/telemetry/protocol/proto',
  '-o', 'src/generated/console.js', '../proto/console.proto', '../proto/execution.proto', '../../apps/cpu-viewer/server/proto/delivery.proto',
], { stdio: 'inherit' });
writeFileSync('src/generated/console.js', readFileSync('src/generated/console.js', 'utf8').replace(
  'import * as $protobuf from "protobufjs/minimal";', 'import $protobuf from "protobufjs/minimal.js";',
));
execFileSync('node_modules/.bin/pbts', [
  '-o', 'src/generated/console.d.ts', 'src/generated/console.js',
], { stdio: 'inherit' });
