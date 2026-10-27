import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';
import {doramcap} from '../frontend/src/generated/workflow.js';
const base=process.env.DEMO_URL??'http://127.0.0.1:8092';
const packets:any[]=[];
const sleep=(ms:number)=>new Promise(resolve=>setTimeout(resolve,ms));
async function waitFor(check:()=>boolean|Promise<boolean>){const until=Date.now()+20000;while(Date.now()<until){if(await check())return;await sleep(30)}throw Error('live validation timeout')}
function connect(){const ws=new WebSocket(base.replace(/^http/,'ws')+'/events');ws.binaryType='arraybuffer';ws.onmessage=message=>{if(typeof message.data!=='string')packets.push(doramcap.workflow.v1.Event.toObject(doramcap.workflow.v1.Event.decode(new Uint8Array(message.data as ArrayBuffer)),{longs:String,enums:String}))};return ws}
const status=async()=>await(await fetch(base+'/api/status')).json();
const run=async(inject_failure:boolean)=>fetch(base+'/api/run',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({inject_failure})});
// Arrange
let ws=connect();await waitFor(()=>ws.readyState===WebSocket.OPEN);await waitFor(async()=>(await status()).ready);
const reports=[];
for(const inject of [false,true]){
 const before=packets.length;
 // Act
 assert.equal((await run(inject)).status,202);
 assert.equal((await run(false)).status,409);
 await waitFor(()=>packets.length>before&&packets.slice(before).some(p=>p.state==='STARTED'));
 // Assert: telemetry arrives while work is still executing.
 assert.equal((await status()).busy,true);
 const session=packets.at(-1).session;
 if(inject)ws.close(); // A disconnected viewer must not cancel cleanup.
 await waitFor(async()=>{const s=await status();return !s.busy&&s.last?.session===session});
 const result=await status();
 assert.equal(result.last.error,null);assert.equal(result.last.dropped_events,0);
 assert.equal(result.last.report.boundary_clean,true);
 assert.equal(result.last.report.recording_finalized,true);
 assert.equal(result.last.report.filesystem_results.unmounted,true);
 assert.equal(result.last.report.filesystem_results.disposition,"write_back");
 if(inject)assert.match(result.last.report.execution_error,/unhealthy/);else assert.equal(result.last.report.execution_error,null);
 for(const key of ['host_binds','container_layers','named_volumes']) {
  assert.ok(result.last.report.filesystem_changes[key].counts.added>0);
  assert.ok(result.last.filesystem_preview[key].rows.some((r:any)=>r.kind==='added'));
 }
 const host=result.last.filesystem_preview.host_binds.rows;
 assert.ok(host.some((r:any)=>r.kind==='modified'&&r.path==='settings.txt'));
 assert.ok(host.some((r:any)=>r.kind==='deleted'&&r.path==='obsolete.txt'));
 assert.equal(result.reports.at(-1).session,session);
 reports.push(result.last);
}
// Act: reconnect and recover bounded replay, including failed and cleanup steps.
packets.length=0;ws=connect();await waitFor(()=>packets.length>=40);
// Assert
assert.equal(new Set(packets.map(p=>p.sequence)).size,packets.length);
assert.ok(packets.some(p=>p.state==='FAILED'));
assert.ok(packets.some(p=>p.step.endsWith('CollectFilesystemChanges')&&p.state==='SUCCEEDED'));
assert.ok(packets.some(p=>p.step.endsWith('PrepareOverlays')&&p.state==='SUCCEEDED'));
assert.ok(packets.some(p=>p.step.endsWith('FinalizeOverlays')&&p.state==='SUCCEEDED'));
assert.ok(packets.some(p=>p.dependencies?.length));
ws.close();
const evidence={events:packets.length,sessions:new Set(packets.map(p=>p.session)).size,live_before_completion:true,reconnect_replay:true,disconnect_cleanup:true,reports};
writeFileSync('/tmp/orchestrator-live-validation.json',JSON.stringify(evidence,null,2)+'\n');
console.log(JSON.stringify(evidence,null,2));
