import assert from 'node:assert/strict';
import { clockSample, browserDelay, type ClockSample } from '../frontend/src/latency';
import { StreamDecoder } from '../frontend/src/protocol';
const base = process.argv[2] ?? 'http://127.0.0.1:8080';
let best: ClockSample | undefined;
for (let i=0;i<3;i++) {
 const start=performance.now();
 const response=await fetch(`${base}/clock`,{cache:'no-store'});
 assert.equal(response.status,200);
 assert.equal(response.headers.get('cache-control'),'no-store');
 const sample=clockSample(await response.text(),start,performance.now());
 if(!best || sample.uncertaintyMs<best.uncertaintyMs) best=sample;
}
const decoder=new StreamDecoder();
const url=new URL('/stream',base);url.protocol='ws:';
await new Promise<void>((resolve,reject)=>{
 const ws=new WebSocket(url);ws.binaryType='arraybuffer';
 let count=0;
 const timer=setTimeout(()=>{ws.close();reject(new Error('timeout'));},5000);
 ws.onerror=()=>{clearTimeout(timer);reject(new Error('socket error'));};
 ws.onmessage=e=>{const received=performance.now();try{
  const s=decoder.receive(new Uint8Array(e.data));if(!s)return;
  const latency=browserDelay(s.capture_unix_ns,received,best!);
  assert.notEqual(latency,null);
  if(++count===10){console.log(JSON.stringify({result:'PASS',samples:count,backendMs:s.viewer_ingress?.capture_to_receive_ms,browserEstimateMs:latency,uncertaintyMs:best!.uncertaintyMs}));clearTimeout(timer);ws.close();resolve();}
 }catch(error){clearTimeout(timer);ws.close();reject(error);}};
});
