import { createRequire } from 'node:module';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const addon = require('../build/console.node') as { open(path: string, version: number): Promise<unknown> };

test('rejects an incompatible contract before reading configuration', async () => {
  // Arrange
  const incompatibleVersion = 999;
  // Act / Assert
  await assert.rejects(addon.open('/does-not-exist', incompatibleVersion), /version mismatch/);
});

import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { once } from 'node:events';

interface ConsoleHandle { close(): Promise<void>; status(): Promise<Buffer> }
const native = addon as { open(path: string, version: number): Promise<ConsoleHandle> };

test('opens and closes twice while Node remains responsive during native startup', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-addon-'));
  const socket = join(directory, 'docker.sock');
  const config = join(directory, 'console.pbtxt');
  const server = createServer((_request, response) => {
    setTimeout(() => { response.writeHead(200, { 'content-type': 'application/json' }); response.end('{"ApiVersion":"1.45"}'); }, 300);
  });
  server.listen(socket);
  await once(server, 'listening');
  await writeFile(config, `listen_address: "127.0.0.1:8080"\npublic_origin: "http://127.0.0.1:8080"\nproject_root: "/tmp"\ndata_root: "${directory}"\ndocker_socket: "${socket}"\naccess_secret_file: "/tmp/secret"\nsystem_apps { cpu_monitor { image: "monitor:local" } }\n`);
  const heartbeat = createServer((_request, response) => response.end('ok'));
  heartbeat.listen(0, '127.0.0.1');
  await once(heartbeat, 'listening');
  const address = heartbeat.address();
  assert.ok(address && typeof address !== 'string');
  let polling = true;
  const latencies: number[] = [];
  const heartbeatLoop = (async () => {
    while (polling) {
      const started = Date.now();
      const response: Response = await fetch(`http://127.0.0.1:${address.port}`);
      assert.equal(await response.text(), 'ok');
      latencies.push(Date.now() - started);
      await new Promise(resolve => setTimeout(resolve, 100));
    }
  })();
  try {
    // Act
    const first = await native.open(config, 1);
    const { doramcap } = require('../src/generated/console.cjs');
    const snapshot = doramcap.console.v1.Status.decode(await first.status());
    assert.equal(snapshot.phase, doramcap.console.v1.Phase.IDLE);
    await first.close();
    const second = await native.open(config, 1);
    await second.close();
    // Assert
    polling = false;
    await heartbeatLoop;
    assert.ok(latencies.length >= 4, `received ${latencies.length} heartbeats`);
    assert.ok(latencies.every(latency => latency < 1000));
  } finally {
    polling = false;
    await heartbeatLoop;
    heartbeat.close();
    await once(heartbeat, 'close');
    server.close();
    await once(server, 'close');
    await rm(directory, { recursive: true, force: true });
  }
});

test('generated codec preserves uint64 values beyond JavaScript integer precision', () => {
  // Arrange
  const { doramcap } = require('../src/generated/console.cjs');
  const value = '9007199254740993';
  // Act
  const encoded = doramcap.console.v1.Status.encode({ sequence: value, steps: [{ invocation: value }] }).finish();
  const decoded = doramcap.console.v1.Status.decode(encoded);
  // Assert
  assert.equal(decoded.sequence.toString(), value);
  assert.equal(decoded.steps[0].invocation.toString(), value);
});

test('artifact descriptors stay beneath the current run and reject symlinks', async () => {
  // Arrange
  const { mkdir, symlink, unlink } = await import('node:fs/promises');
  const { readSync, closeSync } = await import('node:fs');
  const directory = await mkdtemp(join(tmpdir(), 'console-artifact-'));
  const socket = join(directory, 'docker.sock');
  const config = join(directory, 'console.pbtxt');
  const runId = 'a'.repeat(32);
  const runDir = join(directory, 'runs', runId);
  await mkdir(runDir, { recursive: true });
  await writeFile(join(runDir, 'cpu.mcap'), 'evidence');
  const { doramcap } = require('../src/generated/console.cjs');
  await writeFile(join(directory, 'current.pb'), doramcap.console.v1.Marker.encode({
    version: 1, runId, phase: doramcap.console.v1.Phase.FINISHED,
    outcome: { boundaryClean: true, recordingFinalized: true },
  }).finish());
  const server = createServer((_request, response) => { response.writeHead(200, { 'content-type': 'application/json' }); response.end('{"ApiVersion":"1.45"}'); });
  server.listen(socket);
  await once(server, 'listening');
  await writeFile(config, `listen_address: "127.0.0.1:8080"\npublic_origin: "http://127.0.0.1:8080"\nproject_root: "/tmp"\ndata_root: "${directory}"\ndocker_socket: "${socket}"\naccess_secret_file: "/tmp/secret"\nsystem_apps { cpu_monitor { image: "monitor:local" } }\n`);
  const handle = await native.open(config, 1) as ConsoleHandle & { openArtifact(id: string, kind: string): Promise<number> };
  try {
    // Act
    const fd = await handle.openArtifact(runId, 'cpu');
    await unlink(join(runDir, 'cpu.mcap'));
    await symlink('/etc/passwd', join(runDir, 'cpu.mcap'));
    const bytes = Buffer.alloc(8);
    const count = readSync(fd, bytes, 0, bytes.length, 0);
    closeSync(fd);
    // Assert
    assert.equal(bytes.subarray(0, count).toString(), 'evidence');
    await assert.rejects(handle.openArtifact(runId, 'cpu'));
    assert.throws(() => handle.openArtifact('../etc/passwd', 'cpu'));
    await assert.rejects(handle.openArtifact('b'.repeat(32), 'cpu'));
  } finally {
    await handle.close();
    server.close();
    await once(server, 'close');
    await rm(directory, { recursive: true, force: true });
  }
});

test('close waits for in-flight preparation without blocking Node', async () => {
  // Arrange
  const { mkdir, chmod } = await import('node:fs/promises');
  const directory = await mkdtemp(join(tmpdir(), 'console-close-'));
  const socket = join(directory, 'docker.sock');
  const config = join(directory, 'console.pbtxt');
  const project = join(directory, 'project');
  const bin = join(directory, 'bin');
  await mkdir(project); await mkdir(bin);
  await writeFile(join(project, 'compose.yaml'), 'services: {}\n');
  await writeFile(join(bin, 'docker'), '#!/bin/sh\nsleep 1\nexit 1\n');
  await chmod(join(bin, 'docker'), 0o755);
  const server = createServer((_request, response) => { response.writeHead(200, { 'content-type': 'application/json' }); response.end('{"ApiVersion":"1.45"}'); });
  server.listen(socket); await once(server, 'listening');
  await writeFile(config, `listen_address: "127.0.0.1:8080"\npublic_origin: "http://127.0.0.1:8080"\nproject_root: "${directory}"\ndata_root: "${directory}/data"\ndocker_socket: "${socket}"\naccess_secret_file: "/tmp/secret"\nsystem_apps { cpu_monitor { image: "monitor:local" } }\n`);
  const originalPath = process.env.PATH;
  process.env.PATH = `${bin}:/usr/bin:/bin`;
  const handle = await native.open(config, 1) as ConsoleHandle & { prepare(input: Buffer): Promise<Buffer> };
  const { doramcap } = require('../src/generated/console.cjs');
  const request = doramcap.console.v1.PrepareRequest.encode({ directory: project, profiles: [] }).finish();
  const preparation = handle.prepare(Buffer.from(request)).catch(() => null);
  try {
    // Act: wait until the owner has begun the slow preparation, then close.
    for (let i = 0; i < 100; i++) {
      const status = doramcap.console.v1.Status.decode(await handle.status());
      if (status.phase === doramcap.console.v1.Phase.PREPARING) break;
      await new Promise(resolve => setTimeout(resolve, 10));
    }
    const started = Date.now();
    let heartbeat = false;
    const timer = setTimeout(() => { heartbeat = true; }, 100);
    const firstClose = handle.close();
    const secondClose = handle.close();
    await Promise.all([firstClose, secondClose]);
    clearTimeout(timer);
    await preparation;
    // Assert
    assert.ok(Date.now() - started >= 300);
    assert.equal(heartbeat, true);
  } finally {
    process.env.PATH = originalPath;
    server.close(); await once(server, 'close');
    await rm(directory, { recursive: true, force: true });
  }
});
