import { strict as assert } from 'node:assert';
import { readFile, mkdir, mkdtemp, cp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { isAbsolute, join } from 'node:path';
import { capabilities, execution, holoscan, launch } from '../../../console/server/src/native.js';
import { sourceState } from '../../../console/server/src/records/source.js';

const [cli, source, application, imageId, socket] = process.argv.slice(2);
assert.ok([cli,source,socket].every(value=>value && isAbsolute(value)) && application && imageId,
  'usage: tsx prepare-e2e.mts /absolute/holoscan /absolute/project application sha256:image /absolute/docker.sock');
const fixture = await mkdtemp(join(tmpdir(),'holoscan-review-real-'));
const app = join(fixture,'application'), module = join(fixture,'module');
const metadata = await readFile(join(source,'applications',application,'metadata.json'));
await mkdir(app);await writeFile(join(app,'metadata.json'),metadata);
await mkdir(join(module,'applications',application),{recursive:true});
await cp(join(source,'applications',application),join(module,'applications',application),{recursive:true});
await writeFile(join(module,'metadata.json'),JSON.stringify({$schema:'urn:holoscan:module:v2',module:{name:'holoscan-sample',version:'1.0.0',support_level:'community',description:'fixture',authors:[{name:'Fixture',affiliation:'Fixture'}],license:'Apache-2.0',language:['C++'],namespace:{cpp:'holoscan_sample'},holoscan_sdk:{minimum_required_version:'5.0.0',tested_versions:['5.0.0']},platforms:['x86_64'],tags:['fixture'],operator_names:['Sample'],dependencies:[],dockerfile:'Dockerfile'}}));
await writeFile(join(module,'Dockerfile'),'FROM scratch\n');
await writeFile(join(module,'pyproject.toml'),'[tool.holoscan]\ndocker-run-args=["--label=holoscan-review-fixture"]\n');
const owner=await capabilities.open(socket,1);
try {
  for(const root of [source,app,module]) {
    const selection={application,language:'C++',mode:''};
    const request=execution.RunRequest.create({
      run:{id:'real-review',repositoryRoot:root,artifacts:join(fixture,'artifacts'),readinessSeconds:30,stopSeconds:10,hostShmAccess:launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH},
      holoscan:{plan:{cli,selection},buildRecord:{selection,imageId},currentSource:await sourceState(root)},
    });
    const observe=async()=>holoscan.Preparation.fromObject((await owner.inspect(request)).holoscan!);
    const first=await observe(),second=await observe();
    assert.deepEqual(first.rejections,[],`${root}: ${JSON.stringify(first)}`);
    assert.equal(first.snapshot!.sha256,second.snapshot!.sha256);
    assert.equal(first.snapshot!.imageId,imageId);
    assert.equal(first.recordedImage!.state,holoscan.RecordedImageState.RECORDED_IMAGE_STATE_PRESENT);
    assert.equal(first.build!.state,holoscan.BuildState.BUILD_STATE_UNVERIFIED);
    assert.ok(first.unknown.some(item=>item.includes('default mode')));
    assert.ok(first.unknown.some(item=>item.includes('next-launch image')));
    assert.equal(first.diagnostics.at(-1)!.exitCode,0);
    assert.ok(first.diagnostics.at(-1)!.stdout!.length);
    if(root===module)assert.deepEqual(first.snapshot!.projectInputs!.dockerRunArgs!.values,['--label=holoscan-review-fixture']);
  }
  console.log('CLI5 source/Application/Module Review: stable verified snapshots, recorded image presence, explicit unknown defaults and partial dry-run evidence passed');
} finally {await owner.close();await rm(fixture,{recursive:true,force:true});}
