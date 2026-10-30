import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--no-deps', '--offline', '--format-version', '1'], { encoding: 'utf8' })) as { target_directory: string };
mkdirSync('build', { recursive: true });
copyFileSync(join(metadata.target_directory, 'ci', 'libconsole_native.so'), 'build/console.node');
