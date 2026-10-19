import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { decodeSnapshot, decodePacket, StreamDecoder, wire } from './protocol';
const fixture = readFileSync(new URL('./fixtures/snapshot-v2.pb', import.meta.url));
test('decodes Rust Protobuf with lossless u64 and optional metric presence', () => {
 const s=decodeSnapshot(fixture);
 assert.equal(s.sequence,'18446744073709551615');
 assert.equal(s.capture_unix_ns,'1800000000000000001');
 assert.equal(s.records[0].runtime_ns,'18446744073709551615');
 assert.equal(s.task_records![0].identity.start_boottime_ns,'18446744073709551615');
 assert.equal(s.viewer_ingress?.total_bytes,'18446744073709551615');
 assert.equal(s.viewer_ingress?.bytes_per_second,1234.5);
 assert.equal(s.viewer_ingress?.capture_to_receive_ms,null);
});
test('accepts unknown additive fields and rejects malformed or incompatible data', () => {
 assert.equal(decodeSnapshot(Buffer.concat([fixture,Buffer.from([0xa0,0x06,0x01])])).sequence,'18446744073709551615');
 assert.throws(()=>decodeSnapshot(fixture.subarray(0,fixture.length-1)));
 assert.throws(()=>decodeSnapshot(Buffer.concat([fixture,Buffer.from([0])])));
 assert.throws(()=>decodeSnapshot(new Uint8Array()));
 const f=decodePacket(fixture); f.snapshot!.version=999;
 assert.throws(()=>decodeSnapshot(wire.Packet.encode(f).finish()),/version/);
});
test('dictionary topics join by session/revision and preserve root versus unresolved names', () => {
 const decoder=new StreamDecoder(); const f=decodePacket(fixture);
 f.snapshot!.dictionary=[]; f.snapshot!.sessionId='s'; f.snapshot!.dictionaryRevision=2;
 const bytes=()=>wire.Packet.encode(f).finish();
 const dictionary=(sessionId:string,revision:number,path?:string)=>wire.Packet.encode({dictionary:{version:2,sessionId,revision,entries:[{id:9,path}]}}).finish();
 assert.equal(decoder.receive(bytes())!.dictionary.length,0);
 decoder.receive(dictionary('s',1,'old'));assert.equal(decoder.receive(bytes())!.dictionary.length,0);
 decoder.receive(dictionary('s',2,''));assert.equal(decoder.receive(bytes())!.dictionary[0].path,'');
 decoder.receive(dictionary('s',1,'stale'));assert.equal(decoder.receive(bytes())!.dictionary[0].path,'');
 decoder.receive(dictionary('s',3));assert.equal(decoder.receive(bytes())!.dictionary[0].path,null);
 f.snapshot!.sessionId='new';assert.equal(decoder.receive(bytes())!.dictionary.length,0);
});
test('virtual kernel namespace cannot collide with real cgroup zero', () => {
 const f=decodePacket(fixture);
 f.snapshot!.snapshot!.records=[{cgroupId:0,flags:1,runtimeNs:10},{cgroupId:0,flags:129,runtimeNs:20}];
 const s=decodeSnapshot(wire.Packet.encode(f).finish());
 assert.equal(s.records[0].cgroup_id,'0');assert.equal(s.records[1].cgroup_id,'kernel-threads');
});
