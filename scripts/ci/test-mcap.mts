import { run, withTempDirectory } from '../../libs/tooling/tooling.mts';
// Verify shared real-writer recordings with the Rust and TypeScript readers.
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const readerSuite = join(root, 'libs/storage-persistence/tests/readers');

await withTempDirectory('mcap-reader-compatibility-', temporary => {
  const fixtures = join(temporary, 'recordings');
  run('npm', ['ci', '--no-audit', '--no-fund'], { cwd: readerSuite });
  run('cargo', ['run', '--profile', 'ci', '--locked', '-p', 'storage-persistence', '--example', 'reader_compatibility', '--', fixtures], { cwd: root });
  run('npm', ['test', '--', fixtures], { cwd: readerSuite });
});
