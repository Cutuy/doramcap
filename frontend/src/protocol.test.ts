import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { encode, decode } from '@msgpack/msgpack';
import { decodeSnapshot, StreamDecoder } from './protocol';
const fixture = readFileSync(new URL('./fixtures/snapshot-v1.msgpack', import.meta.url));
test('decodes Rust-generated arrays with lossless 64-bit counters', () => {
 const s=decodeSnapshot(fixture);
 assert.equal(s.sequence,'18446744073709551615');
 assert.equal(s.capture_unix_ns,'1800000000000000001');
 assert.equal(s.records[0].runtime_ns,'18446744073709551615');
 assert.equal(s.records[0].cpu,'19');
 assert.equal(s.task_records![0].identity.start_boottime_ns,'18446744073709551615');
 assert.equal(s.task_records![0].charged_runtime_ns,'18446744073709551615');
 assert.equal(s.dictionary[0].path,'test.scope');
 assert.equal(s.viewer_ingress?.total_bytes,'18446744073709551615');
 assert.equal(s.viewer_ingress?.bytes_per_second,1234.5);
});
test('rejects incompatible, truncated and trailing-byte frames',()=>{
 assert.throws(()=>decodeSnapshot(fixture.subarray(0,fixture.length-1)));
 assert.throws(()=>decodeSnapshot(Buffer.concat([fixture,Buffer.from([0])])));
 const frame=decode(fixture,{useBigInt64:true}) as unknown[];
 frame[0]=2;
 assert.throws(()=>decodeSnapshot(encode(frame,{useBigInt64:true})),/version/);
});


test('dictionary topics join by session and revision; unknown revisions never reuse names', () => {
 const decoder = new StreamDecoder();
 const f=decode(fixture,{useBigInt64:true}) as unknown[];
 f[9]=[]; f[6]=2; f[5]='s';
 const bytes=()=>encode(f,{useBigInt64:true});
 const dictionary=(session: string, rev: number, path: string)=>encode([1,session,rev,0,[[9,0,path]],1]);
 assert.equal(decoder.receive(bytes())!.dictionary.length,0);
 assert.equal(decoder.receive(dictionary('s',1,'old')),undefined);
 assert.equal(decoder.receive(bytes())!.dictionary.length,0);
 decoder.receive(dictionary('s',2,'/new'));
 assert.equal(decoder.receive(bytes())!.dictionary[0].path,'/new');
 decoder.receive(dictionary('s',1,'stale'));
 assert.equal(decoder.receive(bytes())!.dictionary[0].path,'/new');
 f[5]='new-session';
 assert.equal(decoder.receive(bytes())!.dictionary.length,0);
});
test('virtual kernel namespace cannot collide with real cgroup ID zero', () => {
 const f=decode(fixture,{useBigInt64:true}) as unknown[];
 (f[1] as unknown[])[7]=[[0,0,0,1,10,0,0],[0,0,0,129,20,0,0]];
 const s=decodeSnapshot(encode(f,{useBigInt64:true}));
 assert.equal(s.records[0].cgroup_id,'0');
 assert.equal(s.records[1].cgroup_id,'kernel-threads');
});
