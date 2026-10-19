import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
execFileSync(root+'node_modules/.bin/pbjs', ['-t','static-module','-w','es6','--no-service','--no-delimited','--no-create','--no-verify','-o',root+'src/generated/telemetry.js',root+'../../../libs/telemetry-protocol/proto/telemetry.proto']);
const file=root+'src/generated/telemetry.js';
writeFileSync(file,readFileSync(file,'utf8').replace('import * as $protobuf from "protobufjs/minimal";', 'import $protobuf from "protobufjs/minimal.js";'));
execFileSync(root+'node_modules/.bin/pbts',['-o',root+'src/generated/telemetry.d.ts',file]);
