#!/usr/bin/node
// Qualification only: run actual rsync unchanged, then stop its real PID during the selected copy.
import { spawn } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = dirname(fileURLToPath(import.meta.url)), args = process.argv.slice(2);
const list = args.indexOf('--files-from');
const selected = list >= 0 && !args.includes('--exclude=/*')
  && readFileSync(args[list + 1]).toString().split('\0').some(path => path.split('/').at(-1) === 'm7-copy-payload');
const child = spawn('/usr/bin/rsync', args, { stdio: 'inherit' });
child.once('spawn', () => {
  if (selected && existsSync(join(root, 'hold-copy'))) {
    if (!child.kill('SIGSTOP')) throw new Error('actual rsync exited before the copy gate');
    writeFileSync(join(root, 'rsync-observed.json'), JSON.stringify({ pid: child.pid, target: args.at(-1) }));
  }
});
child.once('error', error => { console.error(error.message); process.exitCode = 1; });
child.once('exit', (code, signal) => { if (signal) process.kill(process.pid, signal); else process.exit(code ?? 1); });
