import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const exec = promisify(execFile);
const [cli, project] = process.argv.slice(2);
assert.ok(cli && project && isAbsolute(cli) && isAbsolute(project),
  'usage: node discovery-e2e.mts /absolute/holoscan /absolute/holoscan-applications');
const checkout = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const root = await mkdtemp(join(tmpdir(), 'holoscan-discovery-e2e-'));
const discover = async (directory: string, executable = cli, overrides: NodeJS.ProcessEnv = {}) => {
  const { stdout } = await exec('cargo', ['run', '--release', '--locked', '-q', '-p', 'workload-holoscan', '--example', 'catalog', '--', executable, directory],
    { cwd: checkout, env: { ...process.env, ...overrides }, maxBuffer: 16 * 1024 * 1024 });
  return JSON.parse(stdout);
};
try {
  // Arrange: the real upstream project and CLI, with no Docker dependency for discovery.
  const { stdout: version } = await exec(cli, ['--version']);
  assert.match(version, /5\.0\.0/);
  // Act
  const catalog = await discover(project);
  // Assert
  assert.equal(catalog.outcome, 1, catalog.error);
  for (const name of ['bci_visualization', 'endoscopy_tool_tracking']) {
    const entry = catalog.entries.find((entry: any) => entry.name === name && entry.project_type === 'application');
    assert.ok(entry, name);
    assert.ok(isAbsolute(entry.source_folder));
    assert.ok(entry.variants.some((variant: any) => variant.modes.some((mode: any) => mode.name === 'headless' && mode.description)));
  }

  // Arrange / Act / Assert: an ordinary directory is empty; missing CLI is unavailable;
  // missing project is a failure, and each result retains its own cause.
  const empty = await discover(root);
  assert.equal(empty.outcome, 2, empty.error);
  const missingCli = await discover(root, join(root, 'missing-holoscan'));
  assert.equal(missingCli.outcome, 3);
  assert.ok(missingCli.diagnostics[0].error);
  const missingProject = await discover(join(root, 'missing-project'));
  assert.equal(missingProject.outcome, 4);
  assert.match(missingProject.error, /project_root/);

  // Arrange: a native custom search root, a default-mode application, and malformed metadata.
  const metadata = JSON.parse(await readFile(join(project, 'applications/bci_visualization/metadata.json'), 'utf8'));
  delete metadata.application.modes;
  delete metadata.application.default_mode;
  const custom = join(root, 'native-components');
  await mkdir(join(custom, 'sample'), { recursive: true });
  await mkdir(join(custom, 'broken'), { recursive: true });
  await writeFile(join(custom, 'sample/metadata.json'), JSON.stringify(metadata));
  await writeFile(join(custom, 'broken/metadata.json'), '{');
  // Act: inherit the CLI's own search contract; Console adds no search interpretation.
  const mixed = await discover(root, cli, { HOLOSCAN_CLI_SEARCH_PATH: 'native-components' });
  // Assert
  assert.equal(mixed.outcome, 1, mixed.error);
  assert.equal(mixed.entries.length, 1);
  assert.equal(mixed.entries[0].name, 'sample');
  assert.equal(mixed.entries[0].source_folder, join(custom, 'sample'));
  assert.deepEqual(mixed.entries[0].declared_modes, []);
  assert.deepEqual(mixed.entries[0].variants[0].modes, []);
  assert.ok(mixed.diagnostics.some((query: any) => Buffer.from(query.stderr).toString().includes('broken')));
  console.log('CLI 5.0.0 catalog, language modes, empty/missing inputs, native search and mixed metadata: passed');
} finally {
  await rm(root, { recursive: true, force: true });
}
