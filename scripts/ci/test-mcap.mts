// Verify shared real-writer recordings with the Rust and TypeScript readers.
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const readerSuite = join(root, 'libs/storage-persistence/tests/readers');
const temporary = mkdtempSync(join(tmpdir(), 'mcap-reader-compatibility-'));
const run = (command: string, args: string[], cwd: string) => {
  const result = spawnSync(command, args, { cwd, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} failed: ${result.signal ?? result.status}`);
};
try {
  const fixtures = join(temporary, 'recordings');
  run('npm', ['ci', '--no-audit', '--no-fund'], readerSuite);
  run('cargo', ['run', '--profile', 'ci', '--locked', '-p', 'storage-persistence', '--example', 'reader_compatibility', '--', fixtures], root);
  run('npm', ['test', '--', fixtures], readerSuite);
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
