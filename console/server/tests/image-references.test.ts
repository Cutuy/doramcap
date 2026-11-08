import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { imageReferences } from '../src/images.js';

const composeAvailable = spawnSync('docker', ['compose', 'version']).status === 0;

test('image discovery resolves shipped Dora inputs before a runtime directory exists', { skip: !composeAvailable }, async () => {
  // Arrange
  const previous = process.env.DORAMCAP_DORA_RUNTIME_DIR;
  delete process.env.DORAMCAP_DORA_RUNTIME_DIR;
  try {
    for (const role of ['coordinator', 'daemon']) {
      const directory = fileURLToPath(new URL(`../../release/system-apps/dora-${role}/`, import.meta.url));
      // Act: Compose config does not need a running Docker daemon.
      const images = await imageReferences('/nonexistent/docker.sock', { directory, files: ['compose.yaml'] });
      // Assert
      assert.deepEqual(images, ['doramcap/dora:local']);
    }
  } finally {
    if (previous === undefined) delete process.env.DORAMCAP_DORA_RUNTIME_DIR;
    else process.env.DORAMCAP_DORA_RUNTIME_DIR = previous;
  }
});

test('image discovery still rejects other unresolved Compose inputs', { skip: !composeAvailable }, async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-image-inputs-'));
  const variable = 'DORAMCAP_IMAGE_DISCOVERY_TEST_UNSET';
  const previous = process.env[variable];
  delete process.env[variable];
  await writeFile(join(directory, 'compose.yaml'), `services:\n  app:\n    image: 'registry/example:$${'{'}${variable}}'\n`);
  try {
    // Act / Assert
    await assert.rejects(imageReferences('/nonexistent/docker.sock', { directory, files: ['compose.yaml'] }), /Unresolved Compose environment/);
  } finally {
    if (previous === undefined) delete process.env[variable]; else process.env[variable] = previous;
    await rm(directory, { recursive: true, force: true });
  }
});
