import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve} from 'node:path';
import {TraceStore} from '../src/model.js';
import {decodeSnapshot} from '../src/snapshot.js';
const bin=resolve(process.env.BIN_DIR??'../../../target/debug');
const dir=await mkdtemp(`${tmpdir()}/storage-live-`);
const child=spawn(`${bin}/storage-demo`,['--port','0','--dir',`${dir}/session`,'--assets',resolve('dist')]);
let output='',errors='';child.stdout.on('data',b=>output+=b);child.stderr.on('data',b=>errors+=b);
const sockets:WebSocket[]=[];
async function until(check:()=>boolean|Promise<boolean>){const end=Date.now()+20000;while(Date.now()<end){if(await check())return;await new Promise(r=>setTimeout(r,50));}throw Error(`timeout: ${errors}`);}
try{
 await until(()=>output.includes('http://'));
 const base=output.match(/http:\/\/[^\s]+/)![0];
 assert.match(await (await fetch(base)).text(),/\/activity\//);
 assert.match(await (await fetch(base+'/activity/')).text(),/index\.js/);
 assert.equal((await(await fetch(base+'/api/activity/status')).json()).enabled,true);
 let latest:any,wireSize=0;
 const dashboard=new WebSocket(base.replace('http:','ws:')+'/api/snapshots/stream');sockets.push(dashboard);dashboard.binaryType='arraybuffer';
 dashboard.onmessage=e=>{assert.notEqual(typeof e.data,'string');wireSize=e.data.byteLength;latest=decodeSnapshot(new Uint8Array(e.data));};
 const store=new TraceStore();let received=0;
 const connect=(s:TraceStore)=>{const ws=new WebSocket(base.replace('http:','ws:')+'/api/activity/stream');sockets.push(ws);ws.binaryType='arraybuffer';ws.onmessage=e=>{if(typeof e.data!=='string'){s.ingest(new Uint8Array(e.data));received++;}};return ws;};
 const ws=connect(store);
 await until(()=>ws.readyState===WebSocket.OPEN);
 const state=async()=>await(await fetch(base+'/api/state')).json();
 await until(async()=>{const s=await state();assert.notEqual(s.status,'error',JSON.stringify(s));return s.status==='live'&&s.heartbeats>0;});
 const control=async(action:string,extra={})=>assert.equal((await fetch(base+'/api/control',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({action,...extra})})).status,202);
 await control('managed_large',{sizeMiB:4});await control('sealed_slab');
 await until(()=>['combined','large','slab'].every(path=>[...store.spans.values()].some(s=>s.name==='storage.write'&&s.fields.path===path&&s.end!==undefined)));
 assert.ok([...store.spans.values()].some(s=>s.name==='storage.io'&&store.spans.has(s.parent)&&s.end!==undefined));
 assert.ok([...store.spans.values()].some(s=>s.links.length>0));
 for(const path of ['large','slab']){
  const writes=[...store.spans.values()].filter(s=>s.name==='storage.write'&&s.fields.path===path);
  assert.equal(writes.length,1,`${path}: one logical write`);
  const operations=[...store.spans.values()].filter(s=>s.name==='storage.io'&&s.parent===writes[0].id);
  assert.equal(operations.length,1,`${path}: full buffer submitted in one operation`);
  assert.equal(operations[0].fields.len,writes[0].fields.len);
  if(path==='large')assert.ok(Number(operations[0].fields.len)>4*1024*1024);
 }
 console.log('Writer defaults: 4 MiB managed message and sealed slab each use one I/O operation');
 await control('stop');await until(async()=>(await state()).status==='sealed');
 await until(()=>latest?.status==='sealed');
 const json=await state();
 function compare(expected:any,actual:any,path='snapshot'){
  if(expected==null){assert.ok(actual==null,path);return;}
  if(typeof expected!=='object'){assert.equal(actual,expected,path);return;}
  if(Array.isArray(expected))assert.equal(actual.length,expected.length,path);
  for(const [key,value] of Object.entries(expected))compare(value,actual[key],`${path}.${key}`);
 }
 compare(json,latest);
 const jsonSize=Buffer.byteLength(JSON.stringify(json));
 assert.ok(wireSize<jsonSize,`${wireSize} < ${jsonSize}`);
 console.log(`Typed snapshot matches every JSON field: protobuf ${wireSize} B / JSON ${jsonSize} B`);
 await new Promise(r=>setTimeout(r,500));
 ws.close();const before=store.received,packets=received;const again=connect(store);
 await until(()=>received>packets);await new Promise(r=>setTimeout(r,300));
 assert.equal(store.received,before,'replay must not duplicate records');
 again.close();const replay=new TraceStore();connect(replay);await until(()=>replay.received===store.received);
 assert.equal(replay.gaps,0);assert.equal(replay.producerLoss,0);
 console.log(`Live HTTP/WebSocket + replay passed: ${store.received} records, combined/large/slab writes and child I/O`);
}finally{
 child.kill('SIGINT');
 const killTimer=setTimeout(()=>child.kill('SIGKILL'),10000);
 const code=await new Promise(resolve=>{if(child.exitCode!==null)resolve(child.exitCode);else child.once('exit',resolve);});
 clearTimeout(killTimer);for(const ws of sockets)ws.close();
 await rm(dir,{recursive:true,force:true});assert.equal(code,0,errors);
}
