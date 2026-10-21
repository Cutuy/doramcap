import {execFileSync} from 'node:child_process';
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
const root=fileURLToPath(new URL('../',import.meta.url));
mkdirSync(root+'generated', {recursive:true});
const file=root+'generated/wire.js';
execFileSync(root+'node_modules/.bin/pbjs',['-t','static-module','-w','es6','--no-service','-o',file,root+'../proto/viewer.proto',root+'../../../libs/activity-tracing/proto/activity.proto']);
writeFileSync(file,readFileSync(file,'utf8').replace('import * as $protobuf from "protobufjs/minimal";','import $protobuf from "protobufjs/minimal.js";'));
