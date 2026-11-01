import { strict as assert } from 'node:assert';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { ensureImages } from '../src/images.js';
import { compose, wire } from '../src/native.js';

test('preparation downloads selected missing images once and retains cached images', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-images-'));
  const log = join(directory, 'calls.jsonl');
  const previous = process.env.PATH;
  await writeFile(join(directory, 'docker'), `#!${process.execPath} --
const fs = require('node:fs'), args = process.argv.slice(2);
fs.appendFileSync(${JSON.stringify(log)}, JSON.stringify(args)+'\\n');
if (args.includes('compose')) console.log(JSON.stringify({services:{workload:{image:'registry/workload@sha256:new'},cached:{image:'registry/cached@sha256:cached'}}}));
else if (args.includes('inspect')) process.exit(args.at(-1).includes('cached') ? 0 : 1);
else if (args.includes('pull') && args.at(-1).includes('broken')) process.exit(1);
`, { mode: 0o755 });
  process.env.PATH = `${directory}:${previous}`;
  const config = wire.ConsoleConfig.create({ dockerSocket: '/custom/docker.sock', dora: { image: 'registry/dora@sha256:new' } });
  const project = compose.ComposeProject.create({ directory, files: ['compose.yaml'], profiles: ['selected'], envFiles: ['inputs.env'] });
  try {
    // Act
    await ensureImages(config, [project, project]);
    // Assert
    const calls: string[][] = (await readFile(log, 'utf8')).trim().split('\n').map(line => JSON.parse(line));
    assert.ok(calls.every(args => args[1] === 'unix:///custom/docker.sock'));
    assert.deepEqual(calls.filter(args => args.includes('pull')).map(args => args.at(-1)),
      ['registry/dora@sha256:new', 'registry/workload@sha256:new']);
    const resolve = calls.find(args => args.includes('compose'))!;
    assert.ok(resolve.includes('selected') && resolve.includes('inputs.env') && resolve.includes('compose.yaml'));
    // Act / Assert
    config.dora!.image = 'registry/broken@sha256:broken';
    await assert.rejects(ensureImages(config, []), /Image download failed/);
  } finally {
    if (previous === undefined) delete process.env.PATH; else process.env.PATH = previous;
    await rm(directory, { recursive: true, force: true });
  }
});
