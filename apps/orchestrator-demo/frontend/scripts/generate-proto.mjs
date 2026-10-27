import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
mkdirSync(root+'src/generated', {recursive:true});
execFileSync(root+'node_modules/.bin/pbjs', ['-t','static-module','-w','es6','--no-service','--no-delimited','--no-create','--no-verify','-o',root+'src/generated/workflow.js',root+'../../../libs/telemetry/workflow-protocol/proto/workflow.proto']);
const file=root+'src/generated/workflow.js';
writeFileSync(file,readFileSync(file,'utf8').replace('import * as $protobuf from "protobufjs/minimal";', 'import $protobuf from "protobufjs/minimal.js";'));
execFileSync(root+'node_modules/.bin/pbts',['-o',root+'src/generated/workflow.d.ts',file]);
