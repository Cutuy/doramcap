import { execFileSync } from 'node:child_process';
import { cpSync, mkdirSync } from 'node:fs';
execFileSync('node_modules/.bin/tsc', ['-p', 'tsconfig.build.json'], { stdio: 'inherit' });
mkdirSync('dist/generated', { recursive: true });
cpSync('src/generated/console.js', 'dist/generated/console.js');
