import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
mkdirSync(root+'src/generated', {recursive:true});
const file=root+'src/generated/demo.js';
execFileSync(root+'node_modules/.bin/pbjs', ['-t','static-module','-w','es6','--no-service','--no-delimited','--no-create','--no-verify','-p',root+'../../../libs/step-provenance/proto','-p',root+'../../../libs/activity-tracing/proto','-o',file,root+'../server/proto/demo.proto']);
writeFileSync(file,readFileSync(file,'utf8').replace('import * as $protobuf from "protobufjs/minimal";', 'import $protobuf from "protobufjs/minimal.js";'));
execFileSync(root+'node_modules/.bin/pbts',['-o',root+'src/generated/demo.d.ts',file]);
