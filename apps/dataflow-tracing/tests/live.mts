import assert from 'node:assert/strict';
import {Store} from '../frontend/src/model.ts';
const url=process.env.DEMO_URL??'http://127.0.0.1:8090';
const store=new Store();let error:unknown;
const connect=()=>{const ws=new WebSocket(url.replace('http','ws')+'/stream');ws.binaryType='arraybuffer';ws.onmessage=e=>{try{store.ingest(new Uint8Array(e.data as ArrayBuffer));}catch(e){error=e;}};return ws;};
let ws=connect();
try{
 await new Promise(r=>setTimeout(r,4500));if(error)throw error;
 assert.equal(store.nodes.size,4);assert.ok(store.steps.size>30);assert.ok(store.spans.size>30);
 assert.equal(new Set(store.nodes.values()).size,1);
 const matches=[...store.spans.values()].filter(s=>s.name==='step'&&s.end!==undefined).flatMap(span=>{
  const id=store.spanGeneration(span),step=id?store.steps.get(id):undefined;
  if(!step?.end||!step.start)return[];
  const start=store.absolute(step.domain,step.start)!,end=store.absolute(step.domain,step.end)!;
  assert.ok(span.start!<=start&&end<=span.end!,`clock alignment ${span.id}`);return[step];
 });
 assert.ok(matches.length>20);
 const result=[...store.results].reverse().find(r=>r.duration!==undefined&&r.start&&r.end&&store.steps.get(r.anchor)?.inputs.length);assert.ok(result);
 assert.ok(store.ancestors(result.anchor).has(result.start!));
 const received=store.received;ws.close();await new Promise(r=>setTimeout(r,250));ws=connect();
 await new Promise(r=>setTimeout(r,1500));if(error)throw error;
 assert.ok(store.received>received);assert.equal(new Set(store.results.map(r=>r.id)).size,store.results.length);
 console.log(`PASS: live protobuf, four node identities, ${matches.length} aligned step/activity pairs, lineage links, reconnect deduplication`);
}finally{ws.close();}
