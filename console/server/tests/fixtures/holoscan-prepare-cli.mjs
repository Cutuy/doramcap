#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { connect } from 'node:net';
const [flag, root, command, ...args] = process.argv.slice(2);
if (flag !== '--project-root' || root !== process.cwd()) process.exit(12);
const source = join(root, 'app');
const metadata = JSON.parse(readFileSync(join(source, 'metadata.json'), 'utf8')).application;
if (command === 'list') {
  console.log(JSON.stringify({schema_version:1,projects:[{name:metadata.name,project_type:'application',source_folder:source,language:['Python'],modes:Object.keys(metadata.modes)}]}));
} else if (command === 'modes') {
  console.log(JSON.stringify({schema_version:1,project:metadata.name,language:['Python'],modes:metadata.modes}));
} else if (command === 'run') {
  if (!['--no-docker-build','--no-local-build','--persistent','--dryrun'].every(flag => args.includes(flag))) process.exit(13);
  console.log(`[dryrun] generated=${Date.now()} pid=${process.pid} docker run <configured Docker options hidden>`);
  if (existsSync(join(root, 'block'))) {
    const socket = connect(join(root, 'started.sock'), () => socket.write(`${process.pid}\n`));
    await new Promise(() => {});
  }
  if (existsSync(join(root, 'fail'))) { console.error('native dry-run failure retained'); process.exit(7); }
} else process.exit(14);
