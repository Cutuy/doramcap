import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { doramcap } from './generated/console';
import { LiveFrames, mergeLive, decodeStatus } from './live';
import { topCpuGroups } from './cpu';
import type { Snapshot } from './api';
const wire = doramcap.console.v1;
const encode = (value: object) => wire.Status.encode(wire.Status.fromObject(value)).finish();
const snapshot = (value: object) => decodeStatus(encode({runId:'run',phase:wire.Phase.RUNNING,...value}));
test('binary framing survives split headers and bodies with precise u64 IDs', () => {
  // Arrange
  const id = '18446744073709551615';
  const payload = encode({runId:'run',sequence:id,steps:[{invocation:id,state:1}]});
  const frame = new Uint8Array(payload.length + 4); new DataView(frame.buffer).setUint32(0,payload.length,true); frame.set(payload,4);
  const decoder = new LiveFrames(); const received: Snapshot[] = [];
  // Act
  for (const byte of frame) decoder.receive(Uint8Array.of(byte), item => received.push(item));
  // Assert
  assert.equal(received.length,1); assert.equal(received[0].sequence,id); assert.equal(received[0].steps[0].invocation,id);
  assert.equal(received[0].steps[0].state,'STARTED');
  const oversized = new Uint8Array(4); new DataView(oversized.buffer).setUint32(0,4*1024*1024+1,true);
  assert.throws(()=>decoder.receive(oversized,()=>{}),/length/);
});
test('delta merging preserves distinct top-level rows, replaces invocations, bounds metrics and resets runs', () => {
  // Arrange
  let state = snapshot({steps:[{session:'s',executor:'local',invocation:'1',sequence:'1',step: 'compose.prepare', displayName: 'Prepare project',state:2}],cpuSnapshots:[{sequence:'1'}]});
  // Act
  for(let i=2;i<1305;i++) state=mergeLive(state,snapshot({sequence:String(i),steps:[{session:'s',executor:'local',invocation:String(i),sequence:String(i),step:'module::ObserveProjects',state:2}],metrics:[{elapsedNs:String(i)}]}));
  // Assert
  assert.equal(state.steps.length,512); assert.equal(state.steps[0].invocation,'1'); assert.equal(state.metrics?.length,1200); assert.equal(String(state.cpuSnapshots?.[0].sequence),'1');
  // Act
  state=mergeLive(state,snapshot({sequence:'1304',steps:[{session:'s',executor:'local',invocation:'1304',state:3}]}));
  assert.equal(state.steps.at(-1)?.state,'FAILED');
  state=mergeLive(state,snapshot({runId:'new'}));
  assert.equal(state.steps.length,0); assert.equal(state.metrics?.length,0); assert.equal(state.cpuSnapshots?.length,0);
});
test('CPU rankings use existing counters with exact deltas, and suppress reset or missing samples', () => {
  // Arrange
  const counter = 9007199254740993n;
  const make = (sequence:number,delta:bigint) => ({sequence,monotonicEndNs:sequence*100000000,records:[{cgroupId:1,cpu:0,flags:5,runtimeNs:(counter+delta).toString()},{cgroupId:2,cpu:0,flags:65,runtimeNs:'1'},{cgroupId:0,cpu:0,flags:129,runtimeNs:'99'}]});
  // Act
  const source = (sequence:number, delta:bigint) => doramcap.telemetry.v1.Snapshot.fromObject(make(sequence,delta));
  const result=topCpuGroups([source(1,0n),source(2,10000000n)]);
  // Assert
  assert.deepEqual(result,[{id:'1',percent:10}]);
  assert.deepEqual(topCpuGroups([source(1,5n),source(2,0n)]),[]);
  assert.deepEqual(topCpuGroups([]),[]);
});

test('cgroup names keep exact IDs across heartbeats and reset with a fresh CPU frame', () => {
  // Arrange
  const id = '9007199254740993';
  const first = snapshot({cpuSnapshots:[{sequence:'1'}],cpuGroupNames:[{id,name:'worker.service'}]});
  // Act
  const heartbeat = mergeLive(first,snapshot({}));
  const changed = mergeLive(heartbeat,snapshot({cpuSnapshots:[{sequence:'2'}],cpuGroupNames:[]}));
  // Assert
  assert.deepEqual(heartbeat.cpuGroupNames,[{id,name:'worker.service'}]);
  assert.deepEqual(changed.cpuGroupNames,[]);
});

test('review/start sequence resets discard previous-run time even under the same prepared ID', () => {
  // Arrange: an old server published completed activity under the new review ID.
  const stale = snapshot({phase:wire.Phase.REVIEWED,sequence:'800',steps:[{invocation:'1',elapsedNs:'90000000000',state:2}],metrics:[{elapsedNs:'90000000000'}],cpuSnapshots:[{sequence:100}]});
  // Act
  const starting = mergeLive(stale,snapshot({phase:wire.Phase.STARTING,sequence:'0'}));
  // Assert
  assert.equal(starting.steps.length,0);
  assert.equal(starting.metrics?.length,0);
  assert.equal(starting.cpuSnapshots?.length,0);
  // Act / Assert: also reset if the first active frame arrives after Starting.
  const running = mergeLive(stale,snapshot({phase:wire.Phase.RUNNING,sequence:'2',steps:[{invocation:'1',elapsedNs:'100000000',state:1}]}));
  assert.equal(running.steps.length,1);
  assert.equal(running.steps[0].elapsedNs,'100000000');
  // Act / Assert: review frames never inherit activity from an earlier same-ID frame.
  assert.equal(mergeLive(stale,snapshot({phase:wire.Phase.REVIEWED,sequence:'800'})).steps.length,0);
});
