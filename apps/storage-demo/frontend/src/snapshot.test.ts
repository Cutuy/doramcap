import {test} from 'node:test';
import assert from 'node:assert/strict';
import {doramcap} from './generated/snapshot.js';
import {decodeSnapshot} from './snapshot.js';
const wire = doramcap.storage.v1.Snapshot;
const encode = (v:object)=>wire.encode(wire.fromObject({version:1,...v})).finish();
test('snapshot decoder preserves uint64 IDs, unavailable ages and raw file bytes',()=>{
 const result=decodeSnapshot(encode({status:'live',ring:{writer:'9007199254740993',file:'7',oldestUs:null},combining:{used:5,ageMs:null},file:{tail:[0,128,255]},activeWrites:[{write:'18446744073709551615',ageUs:null}],events:[{result:-5}]}));
 assert.equal(result.ring.writer,'9007199254740993');
 assert.equal(result.activeWrites[0].write,'18446744073709551615');
 assert.equal(result.combining.used,5);
 assert.ok(result.combining.ageMs==null);
 assert.ok(result.ring.oldestUs==null);
 assert.ok(result.activeWrites[0].ageUs==null);
 assert.equal(result.events[0].result,-5);
 assert.deepEqual(result.file.tail,[0,128,255]);
});
test('snapshot decoder rejects unknown versions, malformed frames and unsafe counters',()=>{
 assert.throws(()=>decodeSnapshot(encode({version:2})),/version/);
 assert.throws(()=>decodeSnapshot(new Uint8Array([255])));
 assert.throws(()=>decodeSnapshot(encode({cpu:'9007199254740993'})),/Unsafe numeric/);
});
