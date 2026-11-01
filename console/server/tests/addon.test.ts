import { createRequire } from 'node:module';
import { capabilities } from '../src/native.js';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import {ConsoleService} from '../src/service.js';

const require = createRequire(import.meta.url);

test('rejects an incompatible contract before reading configuration', async () => {
  // Arrange
  const incompatibleVersion = 999;
  // Act / Assert
  await assert.rejects(capabilities.open('/does-not-exist', incompatibleVersion), /version mismatch/);
});

import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { once } from 'node:events';

interface ConsoleHandle { close(): Promise<void>; status(): Promise<Buffer> }
const openService=async(path:string)=>(await ConsoleService.open(path)).backend;

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
  await writeFile(join(directory, 'secret'), 'test-secret\n', { mode: 0o600 });
  await writeFile(config, `listen_address: "127.0.0.1:8080"\npublic_origin: "http://127.0.0.1:8080"\nproject_root: "/tmp"\ndata_root: "${directory}"\ndocker_socket: "${socket}"\naccess_secret_file: "${directory}/secret"\ndora { image: "doramcap/dora:local" }\n`);
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
    const first = await openService(config);
    const { doramcap } = require('../src/generated/console.cjs');
    const snapshot = doramcap.console.v1.Status.decode(await first.status());
    assert.equal(snapshot.phase, doramcap.console.v1.Phase.IDLE);
    await first.close();
    const second = await openService(config);
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
    version: 1, runId, artifactReference:runId, phase: doramcap.console.v1.Phase.FINISHED,
    outcome: { boundaryClean: true, recordingFinalized: true }, recordings: [{id: "cpu",path: "cpu.mcap",cpu:true}],
  }).finish());
  const server = createServer((_request, response) => { response.writeHead(200, { 'content-type': 'application/json' }); response.end('{"ApiVersion":"1.45"}'); });
  server.listen(socket);
  await once(server, 'listening');
  await writeFile(join(directory, 'secret'), 'test-secret\n', { mode: 0o600 });
  await writeFile(config, `listen_address: "127.0.0.1:8080"\npublic_origin: "http://127.0.0.1:8080"\nproject_root: "/tmp"\ndata_root: "${directory}"\ndocker_socket: "${socket}"\naccess_secret_file: "${directory}/secret"\ndora { image: "doramcap/dora:local" }\n`);
  const handle = await openService(config) as ConsoleHandle & { openArtifact(id: string, kind: string): Promise<number> };
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
    await assert.rejects(handle.openArtifact('../etc/passwd', 'cpu'));
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
  await writeFile(join(directory, 'secret'), 'test-secret\n', { mode: 0o600 });
  await writeFile(config, `listen_address: "127.0.0.1:8080"\npublic_origin: "http://127.0.0.1:8080"\nproject_root: "${directory}"\ndata_root: "${directory}/data"\ndocker_socket: "${socket}"\naccess_secret_file: "${directory}/secret"\ndora { image: "doramcap/dora:local" }\n`);
  const originalPath = process.env.PATH;
  process.env.PATH = `${bin}:/usr/bin:/bin`;
  const handle = await openService(config) as ConsoleHandle & { prepare(input: Buffer): Promise<Buffer> };
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


test('invalid host configuration fails before creating native state', async () => {
  // Arrange
  const { stat } = await import('node:fs/promises');
  const directory = await mkdtemp(join(tmpdir(), 'console-validation-'));
  const config = join(directory, 'console.pbtxt');
  const data = join(directory, 'data');
  const secret = join(directory, 'secret');
  const text = (origin: string) => `listen_address: "127.0.0.1:8080"
public_origin: "${origin}"
project_root: "${directory}"
data_root: "${data}"
docker_socket: "${directory}/missing.sock"
access_secret_file: "${secret}"`;
  try {
    // Act / Assert: unreadable and empty secrets fail before storage or Docker setup.
    await writeFile(config, text('http://127.0.0.1:8080'));
    await assert.rejects(openService(config), /access_secret_file/);
    await assert.rejects(stat(data), { code: 'ENOENT' });
    await writeFile(secret, '\n', { mode: 0o600 });
    await assert.rejects(openService(config), /nonempty secret/);
    await assert.rejects(stat(data), { code: 'ENOENT' });
    // Act / Assert: backend budget constraints fail before native state is created.
    await writeFile(secret, 'test-secret');
    await writeFile(config, `${text('http://127.0.0.1:8080')}\nruntime { stop_seconds: 301 }`);
    await assert.rejects(openService(config), /stop_seconds/);
    await assert.rejects(stat(data), { code: 'ENOENT' });
    // Act / Assert: an invalid Origin is rejected even with a valid secret.
    await writeFile(secret, 'test-secret');
    await writeFile(config, text('https://example.com/path'));
    await assert.rejects(openService(config), /public_origin/);
    await assert.rejects(stat(data), { code: 'ENOENT' });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('unchanged Start passes preflight while changed commands reject before container creation', async () => {
  // Arrange
  const {mkdir,chmod} = await import('node:fs/promises');
  const root = await mkdtemp(join(tmpdir(),'console-consistency-'));
  const socket=join(root,'docker.sock'), project=join(root,'project'), bin=join(root,'bin');
  await mkdir(project); await mkdir(bin);
  await writeFile(join(project,'compose.yaml'),'services:\n  worker:\n    image: app:local\n');
  const mutations: string[]=[];
  const engine=createServer((request,response)=>{
    const path=request.url??'';response.setHeader('content-type','application/json');
    if(request.method!=='GET'){mutations.push(path);response.writeHead(500);response.end('{}');}
    else if(path.endsWith('/version')) response.end('{"ApiVersion":"1.45"}');
    else if(path.includes('/images/')) response.end('{"Id":"sha256:approved","Config":{}}');
    else if(/\/containers\/json|\/networks(?:\?|$)/.test(path)) response.end('[]');
    else if(/\/volumes(?:\?|$)/.test(path)) response.end('{"Volumes":[]}');
    else {response.writeHead(404);response.end('{"message":"not found"}');}
  });
  engine.listen(socket);await once(engine,'listening');
  const cli=`#!${process.execPath}\nconst fs=require('fs'),a=process.argv.slice(2);if(a.includes('create'))process.exit(43);else if(a.includes('version'))console.log('2.40.0');else if(a.includes('--profiles')||a.includes('--quiet')){}else{const s=fs.readFileSync(a[a.indexOf('-f')+1],'utf8');try{console.log(JSON.stringify(JSON.parse(s)));}catch{console.log(JSON.stringify({services:{worker:{image:'app:local',command:[s.includes('changed')?'changed':'original']}},networks:{default:{}}}));}}\n`;
  await writeFile(join(bin,'docker'),cli);await chmod(join(bin,'docker'),0o755);
  await writeFile(join(root,'secret'),'test-secret');
  const config=join(root,'console.pbtxt');
  await writeFile(config,`listen_address:"127.0.0.1:8080" public_origin:"http://127.0.0.1:8080" project_root:"${root}" data_root:"${root}/data" docker_socket:"${socket}" access_secret_file:"${root}/secret"`);
  const oldPath=process.env.PATH;process.env.PATH=`${bin}:${oldPath}`;
  const handle=await openService(config) as ConsoleHandle & {prepare(input:Buffer):Promise<Buffer>;start(input:Buffer):Promise<Buffer>};
  const {doramcap}=require('../src/generated/console.cjs');const wire=doramcap.console.v1;
  try {
    // Act
    const review=wire.Review.decode(await handle.prepare(Buffer.from(wire.PrepareRequest.encode({directory:project,profiles:[]}).finish())));
    // Assert
    assert.ok(review.services.some((service:any)=>service.name==='worker'&&service.imageId==='sha256:approved'));
    assert.equal(review.systemApps.filter((app:any)=>app.required&&app.selected).length,2);
    assert.deepEqual(mutations,[]);
    // Act / Assert: changing a command must reject before reservation or launch.
    await writeFile(join(project,'compose.yaml'),'services:\n  worker:\n    image: app:local\n    command: changed\n');
    await assert.rejects(handle.start(Buffer.from(wire.StartRequest.encode({preparedId:review.preparedId,requestKey:'changed-command'}).finish())),/StaleReview/);
    assert.deepEqual(mutations,[]);
    // Wait for the rejected run to finish publishing its terminal status.
    let phase=wire.Status.decode(await handle.status()).phase;
    for(let attempt=0;phase!==wire.Phase.IDLE&&attempt<200;attempt++) {
      await new Promise(resolve=>setTimeout(resolve,10));
      phase=wire.Status.decode(await handle.status()).phase;
    }
    assert.equal(phase,wire.Phase.IDLE);
    // Act: a fresh review of unchanged inputs must pass preflight with automatic ports.
    // The fake CLI deliberately fails reservation after the Start acknowledgement.
    const unchanged=wire.Review.decode(await handle.prepare(Buffer.from(wire.PrepareRequest.encode({directory:project,profiles:[]}).finish())));
    const started=wire.StartResponse.decode(await handle.start(Buffer.from(wire.StartRequest.encode({preparedId:unchanged.preparedId,requestKey:'unchanged'}).finish())));
    // Assert
    assert.equal(started.runId,unchanged.runId);
    assert.deepEqual(mutations,[]);
  } finally {
    await handle.close();process.env.PATH=oldPath;engine.close();await once(engine,'close');await rm(root,{recursive:true,force:true});
  }
});
