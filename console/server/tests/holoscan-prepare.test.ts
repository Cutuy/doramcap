import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { createServer as httpServer } from 'node:http';
import { createServer as netServer } from 'node:net';
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { capabilities, execution, holoscan, launch } from '../src/native.js';

const cli = fileURLToPath(new URL('./fixtures/holoscan-prepare-cli.mjs', import.meta.url));
const { SourceState, HostShmAccess } = launch;
const { BuildState } = holoscan;
const image = `sha256:${'a'.repeat(64)}`;

test('native Holoscan Review preserves verified facts, partial evidence and required policy boundaries', {timeout:30_000}, async () => {
  // Arrange: socket Engine fixture, native CLI envelopes, Module defaults and supplied source facts.
  const root = await mkdtemp(join(tmpdir(), 'holoscan-review-'));
  const socket = join(root, 'docker.sock');
  const requests: string[] = [];
  const server = httpServer((request, response) => {
    assert.equal(request.method, 'GET');
    requests.push(request.url!);
    const value = request.url!.endsWith('/version') ? {ApiVersion:'1.41',MinAPIVersion:'1.24',Version:'20.10.24'}
      : request.url!.endsWith('/info') ? {ID:'fixture-daemon',OSType:'linux'}
      : (assert.fail(`Unexpected Engine request ${request.url}`));
    response.writeHead(200, {'Content-Type':'application/json',Connection:'close'});
    response.end(JSON.stringify(value));
  });
  server.listen(socket); await once(server, 'listening');
  const previous = process.env.DOCKER_HOST;
  process.env.DOCKER_HOST = `unix://${socket}`;
  await mkdir(join(root, 'app'));
  const metadata = {application:{name:'sample',language:['Python'],modes:{headless:{run:{command:'sample'}},live:{run:{command:'sample --live'}}}}};
  await writeFile(join(root,'app/metadata.json'),JSON.stringify(metadata));
  await writeFile(join(root,'metadata.json'),JSON.stringify({module:{name:'fixture'}}));
  await writeFile(join(root,'pyproject.toml'),'[tool.holoscan]\ndocker-run-args=["--log-driver=journald"]\n');
  const owner = await capabilities.open(socket,1);
  const base = () => execution.RunRequest.create({
    run:launch.RunPlan.create({id:'review',repositoryRoot:root,artifacts:join(root,'artifacts'),readinessSeconds:5,stopSeconds:5,hostShmAccess:HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH}),
    holoscan:holoscan.PrepareRequest.create({
      buildStoreRoot:join(root,'builds'),
      plan:{cli,selection:{application:'sample',language:'Python',mode:'headless'}},
      buildRecord:{selection:{application:'sample',language:'Python',mode:'headless'},imageId:image,source:{sourceCommit:'abc',sourceState:SourceState.SOURCE_STATE_CLEAN}},
      currentSource:{sourceCommit:'abc',sourceState:SourceState.SOURCE_STATE_CLEAN},
    }),
  });
  const inspect = async (request = base()) => holoscan.Preparation.fromObject((await owner.inspect(request)).holoscan!);
  try {
    // Act / Assert: human PID/time output differs while verified stable input identity stays equal.
    const first = await inspect(), second = await inspect();
    assert.ok(requests.filter(path=>path.endsWith('/version')).length>=4,'Both Engine clients negotiate before observations');
    assert.ok(first.rejections.some(item=>item.includes('No Build record')));
    assert.equal(first.snapshot!.sha256,second.snapshot!.sha256);
    assert.notDeepEqual(first.diagnostics.at(-1)!.stdout,second.diagnostics.at(-1)!.stdout);
    assert.equal(first.recordedImage,null);
    assert.equal(first.snapshot!.imageId,'');
    assert.equal(first.snapshot!.repositoryRoot,root);
    assert.equal(first.snapshot!.cli,cli);
    assert.ok(first.unknown.some(item => item.includes('next-launch image')));
    assert.ok(first.unknown.some(item => item.includes('mounts')));
    assert.ok(first.unknown.some(item => item.includes('log driver')));
    assert.deepEqual(first.snapshot!.projectInputs!.dockerRunArgs!.values,['--log-driver=journald']);
    assert.equal(first.build!.state,BuildState.BUILD_STATE_NOT_BUILT);
    assert.ok(first.warnings.some(item=>item.includes('No Build attempt')));
    // Caller-supplied successful facts cannot replace authoritative missing/latest evidence.
    const forged=base(); forged.holoscan!.buildRecord!.imageId='sha256:forged';
    assert.equal((await inspect(forged)).snapshot!.sha256,first.snapshot!.sha256);
    assert.equal((await owner.buildLookup(holoscan.BuildStoreRequest.create({root:join(root,'builds'),key:{repositoryRoot:root,selection:base().holoscan!.plan!.selection}}))).record,null);
    assert.ok(first.diagnostics.at(-1)!.argv!.includes('--dryrun'));
    assert.ok(!first.diagnostics.at(-1)!.argv!.includes('--img'));
    assert.ok(!first.diagnostics.at(-1)!.argv!.includes('--docker-opts'));
    const fingerprint = first.snapshot!.sha256;
    // Act / Assert: source observations and replacement Build source facts participate independently.
    for (const change of [
      (r: ReturnType<typeof base>) => { r.holoscan!.currentSource!.sourceCommit = 'def'; },
      (r: ReturnType<typeof base>) => { r.holoscan!.currentSource!.sourceState = SourceState.SOURCE_STATE_DIRTY; },
      (r: ReturnType<typeof base>) => { r.holoscan!.plan!.selection!.mode = 'live'; },
      (r: ReturnType<typeof base>) => { r.run!.writePolicy = 2; },
      (r: ReturnType<typeof base>) => { r.run!.stopSeconds = 6; },
    ]) {
      const request = base(); change(request);
      assert.notEqual((await inspect(request)).snapshot!.sha256,fingerprint);
    }
    const dirty = base();
    dirty.holoscan!.buildRecord!.source!.sourceState = SourceState.SOURCE_STATE_DIRTY;
    dirty.holoscan!.currentSource!.sourceState = SourceState.SOURCE_STATE_DIRTY;
    const unverified = await inspect(dirty);
    assert.equal(unverified.build!.state,BuildState.BUILD_STATE_NOT_BUILT);
    assert.ok(unverified.rejections.some(item=>item.includes('No Build record')));
    const missingSource = base(); delete missingSource.holoscan!.buildRecord!.source; delete missingSource.holoscan!.currentSource;
    assert.equal((await inspect(missingSource)).build!.state,BuildState.BUILD_STATE_NOT_BUILT);
    // Act / Assert: requested omissions remain omissions with unknown defaults.
    const defaults = base(); defaults.holoscan!.plan!.selection!.language=''; defaults.holoscan!.plan!.selection!.mode='';
    const partial = await inspect(defaults);
    assert.equal(partial.snapshot!.selection!.mode,'');
    assert.ok(partial.unknown.some(item=>item.includes('default mode')));
    assert.ok(partial.unknown.some(item=>item.includes('default language')));
    // Act / Assert: raw input and selected metadata changes invalidate Review without option parsing.
    await writeFile(join(root,'pyproject.toml'),'[tool.holoscan]\ndocker-run-args=["--log-driver=json-file"]\n');
    assert.notEqual((await inspect()).snapshot!.sha256,fingerprint);
    await writeFile(join(root,'pyproject.toml'),'[tool.holoscan]\ndocker-run-args=["--log-driver=journald"]\n');
    const changed = structuredClone(metadata); changed.application.modes.live.run.command='changed';
    await writeFile(join(root,'app/metadata.json'),JSON.stringify(changed));
    assert.notEqual((await inspect()).snapshot!.sha256,fingerprint);
    await writeFile(join(root,'app/metadata.json'),JSON.stringify(metadata));
    const alias=join(root,'alias'); await symlink(root,alias);
    const aliased=base(); aliased.run!.repositoryRoot=alias;
    assert.equal((await inspect(aliased)).snapshot!.sha256,fingerprint);
    // Act / Assert: supplied volume names and resolved host identity both participate.
    const targetOne=join(root,'target-one'),targetTwo=join(root,'target-two'),one=join(root,'volume-one'),two=join(root,'volume-two');
    await mkdir(targetOne);await mkdir(targetTwo);await symlink(targetOne,one);await symlink(targetOne,two);
    for(const field of ['extra','passthrough'] as const){
      const request=base();
      if(field==='extra')request.holoscan!.plan!.extraDataFolders=[one];else request.run!.passthroughFolders=[one];
      const initial=(await inspect(request)).snapshot!.sha256;
      if(field==='extra')request.holoscan!.plan!.extraDataFolders=[two];else request.run!.passthroughFolders=[two];
      assert.notEqual((await inspect(request)).snapshot!.sha256,initial);
      if(field==='extra')request.holoscan!.plan!.extraDataFolders=[one];else request.run!.passthroughFolders=[one];
      await rm(one);await symlink(targetTwo,one);
      assert.notEqual((await inspect(request)).snapshot!.sha256,initial);
      await rm(one);await symlink(targetOne,one);
    }
    // Act / Assert: mandatory and not-yet-supported policies reject before any CLI/Engine query.
    const before=requests.length;
    for(const change of [
      (r:ReturnType<typeof base>)=>{r.run!.hostShmAccess=HostShmAccess.HOST_SHM_ACCESS_BLOCKED;},
      (r:ReturnType<typeof base>)=>{r.run!.hostShmAccess=HostShmAccess.HOST_SHM_ACCESS_UNSPECIFIED;},
      (r:ReturnType<typeof base>)=>{r.run!.recordings=[{recording:'/tmp/capture.mcap'}];},
      (r:ReturnType<typeof base>)=>{r.run!.systemApps=['unsupported'];},
    ]) {const request=base();change(request);const rejected=await inspect(request);assert.ok(rejected.rejections.length);assert.equal(rejected.snapshot,null);}
    assert.equal(requests.length,before);
    await writeFile(join(root,'fail'),'');
    const dryrunFailed=await inspect();
    assert.ok(dryrunFailed.rejections.some(item=>item.includes('dry run failed')));
    assert.equal(dryrunFailed.diagnostics.at(-1)!.exitCode,7);
    assert.match(Buffer.from(dryrunFailed.diagnostics.at(-1)!.stderr!).toString(),/failure retained/);
    await rm(join(root,'fail'));
    await assert.rejects(owner.start(execution.StartRequest.create({run:base()})),/Holoscan Start is not available yet/);
    // Act / Assert: owner close cancels a blocking dry run and reaps the observable direct child.
    const started=netServer(); started.listen(join(root,'started.sock'));await once(started,'listening');
    const pidPromise=new Promise<number>((resolve,reject)=>{started.once('error',reject);started.once('connection',socket=>{let pid='';socket.on('data',data=>{pid+=data;if(pid.includes('\n'))resolve(Number(pid.trim()));});});});
    await writeFile(join(root,'block'),'');
    const pending=inspect().then(()=>null,error=>error);
    const pid=await pidPromise;
    await owner.close();
    assert.match(String(await pending),/Closing/);
    const CHILD_REAP_DEADLINE_MS=2000, deadline=Date.now()+CHILD_REAP_DEADLINE_MS;
    while(Date.now()<deadline) {try{process.kill(pid,0);}catch(error){if((error as NodeJS.ErrnoException).code==='ESRCH')break;throw error;}await delay(10);}
    assert.throws(()=>process.kill(pid,0),{code:'ESRCH'});
    assert.equal(await readFile(`/proc/${pid}/stat`,'utf8').catch(()=>null),null);
    await new Promise<void>(resolve=>started.close(()=>resolve()));
  } finally {
    await owner.close();
    if(previous===undefined)delete process.env.DOCKER_HOST;else process.env.DOCKER_HOST=previous;
    server.closeAllConnections(); await new Promise<void>(resolve=>server.close(()=>resolve()));
    await rm(root,{recursive:true,force:true});
  }
});
