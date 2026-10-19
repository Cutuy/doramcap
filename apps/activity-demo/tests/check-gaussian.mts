import assert from 'node:assert/strict';
import {TraceStore} from '../frontend/src/model.ts';
const base=process.env.DEMO_URL??'http://127.0.0.1:8088';
const original=await (await fetch(base+'/api/status')).json();
const set=(value:unknown)=>fetch(base+'/api/sleep',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(value)});
const store=new TraceStore();const socket=new WebSocket(base.replace('http','ws')+'/stream');socket.binaryType='arraybuffer';
socket.onmessage=e=>{if(e.data instanceof ArrayBuffer)store.ingest(new Uint8Array(e.data));};
try{
 assert.equal((await set({mu_ms:0,sigma_ms:0})).status,200);
 assert.equal((await set({mu_ms:2001,sigma_ms:0})).status,400);
 assert.equal((await set({mu_ms:0,sigma_ms:1001})).status,400);
 assert.equal((await set({mu_ms:-1,sigma_ms:0})).status,422);
 assert.deepEqual((await (await fetch(base+'/api/status')).json()).sleep,{mu_ms:0,sigma_ms:0});
 await fetch(base+'/api/greet',{method:'POST'});
 await new Promise(r=>setTimeout(r,1500));
 const sampled=[...store.spans.values()].filter(s=>s.name==='sleep'&&s.fields.mu_ms==='0'&&s.fields.sigma_ms==='0');
 assert.ok(sampled.length>=4,`Expected sleep spans from new settings; got ${sampled.length}`);
 for(const span of sampled)assert.equal(span.fields.duration_ms,'0');
 console.log(`Gaussian live check passed: validation, atomic settings, ${sampled.length} traced fixed-duration samples`);
}finally{socket.close();await set(original.sleep);}
