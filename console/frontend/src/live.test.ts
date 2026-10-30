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
test('delta merging preserves lifecycle rows, replaces invocations, bounds metrics and resets runs', () => {
  // Arrange
  let state = snapshot({steps:[{session:'s',executor:'local',invocation:'1',sequence:'1',step:'module::PrepareRun',state:2}],cpuSnapshots:[{sequence:'1'}]});
  // Act
  for(let i=2;i<1305;i++) state=mergeLive(state,snapshot({sequence:String(i),steps:[{session:'s',executor:'local',invocation:String(i),sequence:String(i),step:'module::ObserveProjects',state:2}],metrics:[{elapsedNs:String(i)}]}));
  // Assert
  assert.equal(state.steps.length,512); assert.equal(state.steps[0].invocation,'1'); assert.equal(state.metrics?.length,1200); assert.equal(String(state.cpuSnapshots?.[0].sequence),'1');
  // Act
  state=mergeLive(state,snapshot({steps:[{session:'s',executor:'local',invocation:'1304',state:3}]}));
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
