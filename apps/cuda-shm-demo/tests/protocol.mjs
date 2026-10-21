import {test} from 'node:test';
import assert from 'node:assert/strict';
import {doramcap} from '../frontend/generated/wire.js';
import {decodeSnapshot,encodeConfig} from '../frontend/protocol.mjs';
const pb=doramcap.cuda_viewer.v1, activity=doramcap.activity.v1;
function snapshot(batch) {
  return pb.Snapshot.encode(pb.Snapshot.fromObject({version:1,run:'a',state:'running',cursor:'1',oldest:'1',
    batches:[{id:'1',role:'producer',payload:activity.Batch.encode(activity.Batch.fromObject(batch)).finish()}]})).finish();
}
test('native batch preserves u64 identity and decodes strings and checksums',()=>{
  // Arrange
  const batch={version:1,session:'18446744073709551615',sequence:'2',unixOriginNs:'1780000000000000000',pid:3,
    dictionary:['event','slot','generation','checksum','matches','record'],strings:new TextEncoder().encode('cpu_checksum'),
    records:[{name:5,monotonicNs:'1234567',fields:[{name:0,kind:4,length:12},{name:1,bits:'0'},
      {name:2,bits:'9007199254740993'},{name:3,bits:'247'},{name:4,kind:3,bits:'1'}]}]};
  // Act
  const result=decodeSnapshot(snapshot(batch));
  // Assert
  assert.equal(result.events[0].session,'18446744073709551615');
  assert.equal(result.events[0].fields.generation,'9007199254740993');
  assert.equal(result.events[0].fields.event,'cpu_checksum');
  assert.equal(result.events[0].fields.checksum,247);
  assert.equal(result.events[0].fields.matches,true);
  assert.equal(result.events[0].id,'1/0');
});
test('rejects malformed protobuf, unknown version and invalid string range',()=>{
  // Arrange
  const bad={version:1,session:'1',dictionary:['x'],records:[{fields:[{kind:4,length:99}]}]};
  // Act / Assert
  assert.throws(()=>decodeSnapshot(new Uint8Array([255])));
  assert.throws(()=>decodeSnapshot(pb.Snapshot.encode({version:9}).finish()),/version/);
  assert.throws(()=>decodeSnapshot(snapshot(bad)),/range/);
});
test('run controls roundtrip through protobuf',()=>{
  // Arrange
  const config={frames:24,slots:2,consumers:3,bytes:4097,checksum:true,failure:'stall'};
  // Act
  const result=pb.RunConfig.toObject(pb.RunConfig.decode(encodeConfig(config)));
  // Assert
  assert.deepEqual(result,config);
});
