// Opt-in real Console/Docker acceptance check. Build binaries unprivileged, then run this script with sudo.
import { strict as assert } from 'node:assert';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { cp, mkdir, mkdtemp, readFile, readdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { capabilities, live, wire } from '../server/src/native.js';
import { validateConfig } from '../server/src/config.js';
import { ConsoleService } from '../server/src/service.js';
import { Store } from '../server/src/store.js';
import { releaseConfig } from '../server/src/release.js';
import { availablePort } from '../server/src/launch.js';
import { doramcap } from '../frontend/src/generated/console.js';
import { stageSystemAppInputs } from './system-app-inputs.mts';

const {values} = parseArgs({options:{dora:{type:'string'},'bin-dir':{type:'string'}}});
assert.ok(values.dora && values['bin-dir'], 'usage: --dora DORA --bin-dir TARGET_PROFILE_DIR');
assert.equal(process.getuid?.(), 0, 'Run only this test executable elevated; build binaries unprivileged.');
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..'), bin = resolve(values['bin-dir']);
const directory = await mkdtemp(join(tmpdir(), 'console-system-apps-'));
const tag = `pbtxt-test-${process.pid}`;
console.log(`Acceptance artifacts: ${directory}`);
const images = [
  ['dora','libs/workload-docker/deploy/Dockerfile',resolve(values.dora)],
  ['system-monitor','apps/system-monitor/deploy/Dockerfile',join(bin,'system-monitor')],
  ['mcap-recorder','apps/mcap-recorder/deploy/Dockerfile',join(bin,'mcap-recorder')],
  ['fault-watchdog','apps/fault-watchdog/deploy/standalone/Dockerfile',join(bin,'fault-watchdog')],
  ['live-bridge','console/bridge/Dockerfile',join(bin,'live-bridge')],
];
for (const [name,dockerfile,binary] of images) {
  const context = join(directory, 'images', name);
  await mkdir(context,{recursive:true});
  await cp(binary,join(context,name));
  execFileSync('docker',['build','--quiet','--tag',`doramcap/${name}:${tag}`,'--file',join(root,dockerfile),context],{stdio:'pipe'});
}
const apps = join(directory, 'system-apps');
await cp(join(root,'console/release/system-apps'), apps, {recursive:true});
for (const name of await readdir(apps)) {
  const path = join(apps,name,'compose.yaml');
  await writeFile(path,(await readFile(path,'utf8')).replaceAll(':local',`:${tag}`));
}
stageSystemAppInputs(root, apps);
const template = capabilities.parseConfig((await readFile(join(root,'console/release/console.pbtxt'),'utf8')).replaceAll(':local',`:${tag}`));
let otlpRecords = 0;
const collector = createServer((request,response) => {
  request.resume(); request.on('end',() => {otlpRecords++;response.writeHead(200,{'content-type':'application/x-protobuf'});response.end();});
});
collector.listen(0,'127.0.0.1');
await new Promise<void>(done => collector.once('listening',done));
const address = collector.address() as {port:number};
await writeFile(join(apps,'fault-watchdog/fault-watchdog-otel.pbtxt'),(await readFile(join(apps,'fault-watchdog/fault-watchdog-otel.pbtxt'),'utf8')).replace('127.0.0.1:4318',`127.0.0.1:${address.port}`));

try {
  for (const scenario of ['alone','live','mcap','otel','cpu','faults'] as const) {
    // Arrange: the fixture is a workload participant, separate from the release apps.
    const project = join(directory,scenario,'project'), state = join(directory,scenario,'state');
    await mkdir(project,{recursive:true});
    const withSource = scenario !== 'alone' && scenario !== 'cpu';
    await writeFile(join(project,'compose.yaml'),JSON.stringify({services:{user:withSource ? {
      image:`doramcap/dora:${tag}`,init:true,network_mode:'host',entrypoint:['/fixture'],
      volumes:[{type:'bind',source:join(bin,'examples/health_fixture'),target:'/fixture',read_only:true}],
    } : {image:`doramcap/dora:${tag}`,init:true,entrypoint:['/bin/sleep','infinity']}}}));
    execFileSync('git',['init','--quiet'],{cwd:project});
    execFileSync('git',['add','.'],{cwd:project});
    execFileSync('git',['-c','user.name=Console test','-c','user.email=test@example.invalid','commit','--quiet','-m','Test workload'],{cwd:project});
    const config = releaseConfig(template,project,apps,state);
    config.downloadMissingImages = false;
    config.apps.find(app => app.liveRelay)!.liveRelay!.port = await availablePort();
    if (withSource) {
      const graph = join(directory,scenario,'user.yml');
      await writeFile(graph,JSON.stringify({nodes:[{id:'user',path:'dynamic',outputs:['health','messages']}]}));
      config.dataflowFile = graph;
      const recorder = config.apps.find(app => app.id === 'data-recorder')!;
      const path = join(directory,scenario,'recorder.pbtxt'), flow = join(directory,scenario,'recorder.yml');
      await writeFile(path,'output_path:"/results/messages.mcap" report_path:"/results/messages.report.pbtxt" input {input:"data" topic:"user/messages" message_encoding:"application/octet-stream"}\n');
      await writeFile(flow,JSON.stringify({nodes:[{id:'data-recorder',path:'dynamic',inputs:{data:{source:'user/messages',queue_size:64,queue_policy:'backpressure'}},outputs:['health']}]}));
      recorder.launchProfiles!.push({id:'user',label:'User messages',config:path,graph:flow,recordings:[{id:'messages',path:'messages.mcap'}]});
    }
    validateConfig(config);
    const store = await Store.open(state), engine = await capabilities.open(config.dockerSocket,1), backend = new ConsoleService(config,engine,store);
    const observed = new Set<number>(), fresh = new Set<number>(), components = new Set<string>();
    let snapshots = 0;
    const unsubscribe = backend.frames(bytes => {
      const frame = live.Frame.decode(bytes);
      if (!frame.stream.startsWith('fault-watchdog_')) return;
      const publication = doramcap.fault.v1.AggregationPublication.decode(frame.payload);
      const states = [...(publication.snapshot?.sources.flatMap(source => source.conditions) ?? []), ...(publication.updates?.updates.flatMap(update => update.state ? [update.state] : []) ?? [])];
      if (publication.snapshot) snapshots++;
      for (const condition of states) {
        if (condition.definition) components.add(condition.definition.component);
        if (condition.definition?.component === 'validation') {observed.add(condition.health);fresh.add(condition.freshness);}
      }
    });
    let runId = '';
    try {
      const ids = scenario === 'alone' ? ['fault-watchdog','live-bridge'] : ['cpu-monitor','data-recorder','fault-watchdog','live-bridge'];
      const profiles = {'fault-watchdog':scenario === 'mcap' || scenario === 'otel' ? scenario : 'live', ...(scenario !== 'alone' ? {'data-recorder':scenario === 'faults' ? 'faults' : withSource ? 'user' : 'cpu'} : {})};
      const review = wire.Review.decode(await backend.prepare(Buffer.from(wire.PrepareRequest.encode({systemApps:{ids,profiles}}).finish())));
      assert.ok(review.preparedId); runId = review.runId;
      // Act
      await backend.start(Buffer.from(wire.StartRequest.encode({preparedId:review.preparedId,requestKey:scenario}).finish()));
      const deadline = Date.now()+90_000;
      let runningAt = 0;
      while (Date.now()<deadline) {
        const status = wire.Status.decode(await backend.status());
        if (status.phase === wire.Phase.RUNNING) {
          runningAt ||= Date.now();
          if (Date.now()-runningAt > (withSource ? 9500 : 2500)) break;
        }
        assert.ok(![wire.Phase.BLOCKED,wire.Phase.FINISHED].includes(status.phase),JSON.stringify(status.outcome));
        await new Promise(done => setTimeout(done,100));
      }
      assert.ok(runningAt,'run did not become ready');
      await backend.stop(Buffer.from(wire.StopRequest.encode({runId}).finish()));
      let terminal = wire.Status.decode(await backend.status());
      while (![wire.Phase.FINISHED,wire.Phase.BLOCKED].includes(terminal.phase) && Date.now()<deadline) {
        await new Promise(done => setTimeout(done,100));terminal=wire.Status.decode(await backend.status());
      }
      // Assert
      assert.equal(terminal.phase,wire.Phase.FINISHED,JSON.stringify(terminal.outcome));
      assert.equal(terminal.outcome?.boundaryClean,true,JSON.stringify(terminal.outcome));
      assert.equal(terminal.outcome?.execution,'Completed',JSON.stringify(terminal.outcome));
      if (scenario !== 'alone') assert.equal(terminal.outcome?.recordingFinalized,true,JSON.stringify(terminal.outcome));
      assert.ok(snapshots>0,'watchdog sent no live snapshots');
      if (withSource) {
        assert.ok(observed.has(3),'active fault missing');
        assert.ok(observed.has(2) || observed.has(4),'recovery missing');
        assert.ok(fresh.has(3),'source staleness missing');
        assert.ok(components.has('mcap-recorder'),'recorder health missing');
        const artifacts = join(state,'runs',runId);
        execFileSync(join(bin,'examples/check_messages'),[join(artifacts,'messages.mcap'),scenario === 'faults' ? 'fault-watchdog/faults' : 'user/messages'],{stdio:'inherit'});
        if (scenario === 'mcap') execFileSync(join(bin,'examples/fault_readback'),[join(artifacts,'faults.mcap')],{stdio:'inherit'});
      }
      if (scenario === 'cpu') assert.ok(components.has('system-monitor'),'CPU monitor health missing');
      if (scenario === 'otel') assert.ok(otlpRecords>0,'collector received no records');
      console.log(`${scenario}: live snapshots=${snapshots}, components=${[...components].join(',')}, ${withSource ? 'health failure/recovery/staleness verified' : 'live diagnostics verified'}`);
    } finally {unsubscribe();await backend.close();}
  }
} finally {
  collector.close();
  for (const [name] of images) execFileSync('docker',['image','rm',`doramcap/${name}:${tag}`],{stdio:'pipe'});
}
