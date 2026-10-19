// Run against either live dataflow. Same binary decoder/model as React.
import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { TopCpuStreams } from '../frontend/src/stream';
import { decodeSnapshot, StreamDecoder } from '../frontend/src/protocol';
const base = process.argv[2] ?? 'http://127.0.0.1:8080';
const page = await fetch(base);
assert.equal(page.status, 200);
assert.match(await page.text(), /<title>DORA CPU activity<\/title>/);
const url = new URL('/stream', base); url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
const model = new TopCpuStreams();
const decoder = new StreamDecoder();
let count = 0, first = 0, last = 0, active = false, finalBytes = new Uint8Array(), source = '';
await new Promise<void>((resolve, reject) => {
 const socket = new WebSocket(url); socket.binaryType = 'arraybuffer';
 let done = false;
 const timer = setTimeout(()=>finish(new Error('Stream timeout')),45000);
 function finish(error?: unknown) { if(done)return;done=true;clearTimeout(timer);socket.close();error?reject(error):resolve(); }
 socket.onerror=()=>finish(new Error('WebSocket error'));
 socket.onclose=()=>{if(!done)finish(new Error('Stream closed early'));};
 socket.onmessage=event=>{
  try {
   assert.ok(event.data instanceof ArrayBuffer, 'expected binary frame');
   const bytes = new Uint8Array(event.data), s=decoder.receive(bytes);
   if (!s) return;
   assert.equal(decodeSnapshot(bytes).dictionary.length, 0, 'no names on 10Hz frames');
   assert.equal(s.task_records?.length, 0, 'no per-task records on wire');
   source=s.source;
   // Cross-topic ordering may initially leave names unresolved.
   const now=performance.now()/1000;
   if(!count)first=now; last=now;
   model.ingest(s,now);count++;
   for(const buffer of model.buffers.values()) {
    for(const rate of buffer.rows().at(-1)!.values.values()) if(rate!==null) {assert.ok(rate>=0);active ||= rate>0;}
   }
   finalBytes=bytes;
   if(last-first>=32)finish();
  } catch(error){finish(error);}
 };
});
assert.ok(count>=200, `only ${count} snapshots`);
assert.ok(active,'expected advancing CPU counters');
assert.ok(decoder.dictionaryPackets >= 10 && decoder.dictionaryPackets <= 20, 'dictionary about every 2 seconds');
for(const buffer of model.buffers.values()) {
 assert.ok(buffer.size<=600);
 assert.ok(buffer.rows()[0].time>=last-30);
 assert.ok(buffer.rows()[0].time>first);
}
model.refresh(last+31);assert.equal(model.buffers.size,0);
// A newly connected browser must receive names without waiting for a keyframe.
await new Promise<void>((resolve,reject)=>{
 const reconnectDecoder = new StreamDecoder();
 const socket=new WebSocket(url);socket.binaryType='arraybuffer';
 const timer=setTimeout(()=>{socket.close();reject(new Error('Reconnect timeout'));},5000);
 socket.onerror=()=>{clearTimeout(timer);reject(new Error('Reconnect error'));};
 socket.onmessage=e=>{try{const s=reconnectDecoder.receive(new Uint8Array(e.data));if(!s)return;assert.ok(s.dictionary.length>0);clearTimeout(timer);socket.close();resolve();}catch(error){clearTimeout(timer);socket.close();reject(error);}};
});
if(process.argv[3])writeFileSync(process.argv[3],finalBytes);
const latest=decodeSnapshot(finalBytes);
console.log(JSON.stringify({result:'PASS',source,snapshots:count,dictionaryPackets:decoder.dictionaryPackets,binaryFrameBytes:finalBytes.length,ingress:latest.viewer_ingress,checks:'binary decode, rates, 30s expiry, reconnect names'}));
