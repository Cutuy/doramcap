import assert from 'node:assert/strict';
const base=process.env.DEMO_URL??'http://127.0.0.1:8088';
const status=async()=> (await (await fetch(base+'/api/status')).json());
const post=(path:string,body:unknown)=>fetch(base+path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});
const sleep=(ms:number)=>new Promise(r=>setTimeout(r,ms));
const original=await status();
try {
 assert.equal((await post('/api/playground',{mu_ms:0,sigma_ms:0,frequency_hz:0.1})).status,200);
 await post('/api/control',{paused:false});
 const slow=await status();await sleep(300);
 assert.equal((await status()).conversations,slow.conversations);
 assert.equal((await post('/api/playground',{mu_ms:100,sigma_ms:0,frequency_hz:21})).status,400);
 assert.equal((await status()).sleep.mu_ms,0);
 assert.equal((await post('/api/playground',{mu_ms:0,sigma_ms:0,frequency_hz:20})).status,200);
 await sleep(700);const fast=await status();
 assert.ok(fast.conversations-slow.conversations>=5,'Rate update must wake the ten-second timer promptly');
 assert.equal(fast.frequency_hz,20);
 await post('/api/control',{paused:true});const paused=await status();await sleep(200);
 assert.equal((await status()).conversations,paused.conversations);
 await post('/api/greet',{});assert.equal((await status()).conversations,paused.conversations+1);
 console.log('Frequency check passed: low/high rates, prompt rescheduling, invalid update rejection, pause, manual greeting');
}finally{await post('/api/playground',{...original.sleep,frequency_hz:original.frequency_hz});await post('/api/control',{paused:original.paused});}
