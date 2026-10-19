import assert from 'node:assert/strict';
const base=process.env.DEMO_URL??'http://127.0.0.1:8088';
const status=async()=> (await (await fetch(base+'/api/status')).json());
const post=(path:string,body:unknown)=>fetch(base+path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});
const original=await status();
const socket=new WebSocket(base.replace('http','ws')+'/stream');
try {
 await new Promise<void>((resolve,reject)=>{socket.onopen=()=>resolve();socket.onerror=reject;});
 await post('/api/control',{paused:true});await post('/api/sleep',{mu_ms:2000,sigma_ms:0});
 for(let i=0;i<12;i++)assert.equal((await post('/api/greet',{})).status,200);
 const loaded=(await status()).resources;
 assert.ok(loaded.mailboxes.some((q:{used:number})=>q.used>0));
 assert.ok(loaded.live.clients>=1);
 assert.ok(loaded.rss_bytes>0);assert.ok(loaded.peak_rss_bytes>=loaded.rss_bytes);
 assert.equal(loaded.capture.capacity,128);assert.equal(loaded.encoded.capacity,68);
 assert.ok(loaded.capture.reserved_bytes>0);assert.equal(loaded.encoded.reserved_bytes,68*(64*4096+8192+1024));
 await new Promise(r=>setTimeout(r,2500));
 const later=(await status()).resources;
 assert.ok(later.live.total_bytes>loaded.live.total_bytes);
 assert.ok(later.replay.bytes<=later.replay.byte_capacity);
 assert.ok(later.replay.batches<=later.replay.batch_capacity);
 for(const pool of [later.capture,later.encoded,...later.mailboxes])assert.ok(pool.used>=0&&pool.used<=pool.capacity);
 console.log(`Resource check passed: RSS ${later.rss_bytes} B, pools ${later.capture.reserved_bytes+later.encoded.reserved_bytes} B, mailbox backlog observed, encoded byte counter increasing`);
} finally {socket.close();await post('/api/sleep',original.sleep);await post('/api/control',{paused:original.paused});}
