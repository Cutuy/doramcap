import { doramcap } from '../src/generated/console.js';
import { capabilities, execution, compose, composePlan, launch } from '../src/native.js';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import {ConsoleService} from '../src/service.js';
import { Store } from '../src/store.js';


test('rejects an incompatible contract before reading configuration', async () => {
  // Arrange
  const incompatibleVersion = 999;
  // Act / Assert
  await assert.rejects(capabilities.open('/does-not-exist', incompatibleVersion), /version mismatch/);
});

test('an empty Docker socket opens without Docker and refuses Compose work', async () => {
  // Arrange / Act: no Docker connection is attempted.
  const owner = await capabilities.open('', 1);
  try {
    // Assert
    await assert.rejects(
      owner.composeProfiles(execution.ComposeProfilesRequest.create({ runId: 'r1', repositoryRoot: '/', project: { key: 'workload' } })),
      /Docker is not configured/,
    );
    const run = execution.RunRequest.create({
      run: { hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH, id: 'r1', repositoryRoot: '/', artifacts: '/tmp/console-no-docker', readinessSeconds: 1, stopSeconds: 1 },
      compose: {},
    });
    await assert.rejects(owner.inspect(run), /Docker is not configured/);
    await assert.rejects(
      owner.start(execution.StartRequest.create({ run, composeExpected: compose.ConfigurationSnapshot.create() })),
      /Docker is not configured/,
    );
  } finally {
    await owner.close();
  }
});

import { mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { once } from 'node:events';

const doraApps = ['coordinator', 'daemon'].map((role) => {
  const directory = fileURLToPath(new URL(`../../release/system-apps/dora-${role}/`, import.meta.url));
  return `dora_apps: "dora-${role}" apps {id: "dora-${role}" input {directory: ${JSON.stringify(directory)} files: "compose.yaml"}}`;
}).join('\n');

interface ConsoleHandle { close(): Promise<void>; status(): Promise<Buffer> }
const openService=async(path:string)=>(await ConsoleService.open(path)).backend;

test('native profile listing preserves a missing Compose file rejection', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-profile-rejection-'));
  const socket = join(root, 'docker.sock');
  let mutations = 0;
  const server = createServer((request, response) => {
    if (request.method !== 'GET') mutations++;
    response.setHeader('content-type', 'application/json');
    response.end('{"ApiVersion":"1.45"}');
  });
  server.listen(socket);
  await once(server, 'listening');
  const owner = await capabilities.open(socket, 1);
  try {
    // Act / Assert: profile listing rejects before invoking Compose or creating resources.
    await assert.rejects(owner.composeProfiles(execution.ComposeProfilesRequest.create({
      runId: 'profile-rejection', repositoryRoot: root,
      project: { key: 'workload', directory: root },
    })), (error: Error & { check?: string }) => {
      assert.ok(error.check?.startsWith('libs/compose-apps/src/compose/adapter/configuration.rs:'), String(error.check));
      assert.equal(error.message, 'at least one Compose file is required');
      return true;
    });
    assert.equal(mutations, 0);
  } finally {
    await owner.close();
    server.close();
    await once(server, 'close');
    await rm(root, { recursive: true, force: true });
  }
});

test('opens and closes twice without contacting Docker', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-addon-'));
  const socket = join(directory, 'docker.sock');
  const config = join(directory, 'console.pbtxt');
  let requests = 0;
  const server = createServer((_request, response) => {
    requests++;
    response.setHeader('content-type', 'application/json');
    response.end('{"ApiVersion":"1.45"}');
  });
  server.listen(socket);
  await once(server, 'listening');
  await writeFile(join(directory, 'secret'), 'test-secret\n', { mode: 0o600 });
  await writeFile(config, `host_shm_access: HOST_SHM_ACCESS_PASSTHROUGH listen_address: "127.0.0.1:8080"\npublic_origin: "http://127.0.0.1:8080"\nproject_root: "/tmp"\ndata_root: "${directory}"\ndocker_socket: "${socket}"\naccess_secret_file: "${directory}/secret"\n${doraApps}\n`);
  try {
    // Act
    const first = await openService(config);
    const snapshot = doramcap.console.v1.Status.decode(await first.status());
    assert.equal(snapshot.phase, doramcap.console.v1.Phase.IDLE);
    await first.close();
    const second = await openService(config);
    await second.close();
    // Assert
    assert.equal(requests, 0);
  } finally {
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
  const store = await Store.open(directory);
  store.write(doramcap.console.v1.Marker.create({
    version: 1, runId, artifactReference:runId, phase: doramcap.console.v1.Phase.FINISHED,
    outcome: { boundaryClean: true, recordingFinalized: true }, recordings: [{id: "cpu",path: "cpu.mcap",cpu:true}],
  }));
  store.close();
  const server = createServer((_request, response) => { response.writeHead(200, { 'content-type': 'application/json' }); response.end('{"ApiVersion":"1.45"}'); });
  server.listen(socket);
  await once(server, 'listening');
  await writeFile(join(directory, 'secret'), 'test-secret\n', { mode: 0o600 });
  await writeFile(config, `host_shm_access: HOST_SHM_ACCESS_PASSTHROUGH listen_address: "127.0.0.1:8080"\npublic_origin: "http://127.0.0.1:8080"\nproject_root: "/tmp"\ndata_root: "${directory}"\ndocker_socket: "${socket}"\naccess_secret_file: "${directory}/secret"\n${doraApps}\n`);
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
  await writeFile(config, `host_shm_access: HOST_SHM_ACCESS_PASSTHROUGH listen_address: "127.0.0.1:8080"\npublic_origin: "http://127.0.0.1:8080"\nproject_root: "${project}"\ndata_root: "${directory}/data"\ndocker_socket: "${socket}"\naccess_secret_file: "${directory}/secret"\n${doraApps}\n`);
  const originalPath = process.env.PATH;
  process.env.PATH = `${bin}:/usr/bin:/bin`;
  const handle = await openService(config) as ConsoleHandle & { prepare(input: Buffer): Promise<Buffer> };
  const request = doramcap.console.v1.PrepareRequest.encode({ profiles: [] }).finish();
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
  const text = (origin: string) => `host_shm_access: HOST_SHM_ACCESS_PASSTHROUGH listen_address: "127.0.0.1:8080"
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
  await writeFile(join(project,'compose.yaml'),JSON.stringify({services:{worker:{image:'app:local',command:['original']}},networks:{default:{}}}));
  const mutations: string[]=[];
  const engine=createServer((request,response)=>{
    const path=request.url??'';response.setHeader('content-type','application/json');
    if(request.method!=='GET'){mutations.push(path);response.writeHead(500);response.end('{}');}
    else if(path.endsWith('/version')) response.end('{"ApiVersion":"1.45"}');
    else if(path.endsWith('/info')) response.end('{"OSType":"linux","LoggingDriver":"json-file","Plugins":{"Log":["json-file"]}}');
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
  await writeFile(config,`host_shm_access: HOST_SHM_ACCESS_PASSTHROUGH listen_address:"127.0.0.1:8080" public_origin:"http://127.0.0.1:8080" project_root:"${project}" data_root:"${root}/data" docker_socket:"${socket}" access_secret_file:"${root}/secret"`);
  const oldPath=process.env.PATH;process.env.PATH=`${bin}:${oldPath}`;
  const handle=await openService(config) as ConsoleHandle & {prepare(input:Buffer):Promise<Buffer>;start(input:Buffer):Promise<Buffer>};
  const wire=doramcap.console.v1;
  try {
    // Act / Assert: omitted logging rejects at Review on a host without journald.
    await assert.rejects(handle.prepare(Buffer.from(wire.PrepareRequest.encode({profiles:[]}).finish())), (error: Error & { check?: string; detail?: string }) => {
      assert.equal(error.message, 'InvalidInput');
      assert.ok(error.check?.startsWith('libs/compose-apps/src/compose/adapter/configuration.rs:'), String(error.check));
      assert.ok(error.detail?.includes('worker: Docker does not provide the journald logging driver'));
      return true;
    });
    assert.deepEqual(mutations,[]);
    const diagnosticFiles = await readdir(join(root,'data','diagnostics'));
    assert.ok((await Promise.all(diagnosticFiles.map(file => readFile(join(root,'data','diagnostics',file),'utf8'))))
      .some(text => text.includes('worker: Docker does not provide the journald logging driver')));
    await writeFile(join(project,'compose.yaml'),JSON.stringify({services:{worker:{image:'app:local',command:['original'],logging:{}}},networks:{default:{}}}));
    // Act
    const review=wire.Review.decode(await handle.prepare(Buffer.from(wire.PrepareRequest.encode({profiles:[]}).finish())));
    // Assert
    assert.ok(review.services.some((service:any)=>service.name==='worker'&&service.imageId==='sha256:approved'));
    assert.deepEqual(review.warnings,['worker: logging configured by project; Console will not collect its logs']);
    assert.equal(review.systemApps.filter((app:any)=>app.required&&app.selected).length,0);
    assert.deepEqual(mutations,[]);
    // Act / Assert: changing a command must reject before reservation or launch.
    await writeFile(join(project,'compose.yaml'),JSON.stringify({services:{worker:{image:'app:local',command:['changed'],logging:{}}},networks:{default:{}}}));
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
    const unchanged=wire.Review.decode(await handle.prepare(Buffer.from(wire.PrepareRequest.encode({profiles:[]}).finish())));
    const started=wire.StartResponse.decode(await handle.start(Buffer.from(wire.StartRequest.encode({preparedId:unchanged.preparedId,requestKey:'unchanged'}).finish())));
    // Assert
    assert.equal(started.runId,unchanged.runId);
    assert.deepEqual(mutations,[]);
  } finally {
    await handle.close();process.env.PATH=oldPath;engine.close();await once(engine,'close');await rm(root,{recursive:true,force:true});
  }
});


test('native Start preserves a budget rejection and publishes its terminal facts before rejecting', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-admission-'));
  const socket = join(root, 'docker.sock');
  let mutations = 0;
  const server = createServer((request, response) => {
    if (request.method !== 'GET') mutations++;
    response.setHeader('content-type', 'application/json');
    response.end('{"ApiVersion":"1.45"}');
  });
  server.listen(socket);
  await once(server, 'listening');
  const owner = await capabilities.open(socket, 1);
  const request = execution.RunRequest.create({
    compose: composePlan.ComposePlan.create(),
    run: launch.RunPlan.create({
      hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH,
      id: 'rejected-budget', repositoryRoot: root, artifacts: join(root, 'artifacts'),
      readinessSeconds: 0, stopSeconds: 1,
    }),
  });
  try {
    // Act
    let rejection: Error | undefined;
    await assert.rejects(owner.start(execution.StartRequest.create({ run: request, composeExpected: compose.ConfigurationSnapshot.create() })), (error: Error) => {
      rejection = error;
      return /readiness_seconds/.test(error.message);
    });
    const update = owner.poll();
    // Assert
    assert.equal(update.runId, request.run!.id);
    assert.equal(update.result!.error, rejection!.message);
    assert.equal(update.result!.blocked, false);
    assert.equal(mutations, 0);
  } finally {
    await owner.close();
    server.close();
    await once(server, 'close');
    await rm(root, { recursive: true, force: true });
  }
});
