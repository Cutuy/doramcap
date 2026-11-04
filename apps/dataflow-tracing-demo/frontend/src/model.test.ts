import test from 'node:test';
import assert from 'node:assert/strict';
import {Envelope,Store,key,zoom} from './model';
const r=(step:string,n=1)=>({producer:step,epoch:'epoch',step,generation:String(n),sequence:String(n)});
const frame=(seq:number,body:object,stream='monitor-1')=>Envelope.encode(Envelope.fromObject({version:1,sequence:String(seq),streamId:stream,...body})).finish();
test('split step boundaries, exact u64 values, gaps and replay deduplication',()=>{
 const s=new Store(),ref=r('camera');
 s.ingest(frame(1,{clock:{session:'1',unixOriginNs:'1800000000000000000',domain:'clock'}}));
 const begin=frame(2,{step:{generation:ref,clockDomain:'clock',startNs:'9007199254740993',inputs:{refs:[]}}});
 assert.ok(s.ingest(begin));assert.equal(s.ingest(begin),false);
 s.ingest(frame(4,{step:{generation:ref,clockDomain:'clock',endNs:'9007199254741000'}}));
 assert.equal(s.steps.get(key(ref))!.end!-s.steps.get(key(ref))!.start!,7n);assert.equal(s.gaps,1);
 s.ingest(frame(5,{gap:{envelopes:'1'}}));
 s.ingest(frame(6,{status:{node:'camera',pid:123}}));assert.equal(s.gaps,2);
});
test('late clock resolves result timestamps; monitor restart resets stores',()=>{
 const s=new Store();s.ingest(frame(1,{result:{measurement:'latency',anchor:r('detect'),status:1,durationNs:'10',end:{generation:r('detect'),timestampNs:'50',clockDomain:'clock'}}}));
 assert.equal(s.results[0].time,undefined);
 s.ingest(frame(2,{clock:{domain:'clock',unixOriginNs:'1000'}}));assert.equal(s.results[0].time,1050n);
 s.ingest(frame(1,{status:{node:'camera',pid:456}},'monitor-2'));assert.equal(s.results.length,0);assert.equal(s.nodes.get('camera'),456);
});
test('ancestry and nested activity spans resolve to the same generation',()=>{
 const s=new Store();const ref=r('detect');
 s.ingest(frame(1,{step:{generation:ref,clockDomain:'clock',startNs:'0',inputs:{refs:[r('camera'),r('lidar')]}}}));
 s.ingest(frame(2,{activity:{version:1,session:'5',unixOriginNs:'1000',dictionary:['step','detect','producer','epoch','generation','sequence','compute'],strings:Buffer.from('detectepochdetect'),records:[
  {kind:0,spanId:'10',name:0,threadName:1,monotonicNs:'0',fields:[{name:2,kind:4,offset:0,length:6},{name:3,kind:4,offset:6,length:5},{name:0,kind:4,offset:11,length:6},{name:4,kind:0,bits:'1'},{name:5,kind:0,bits:'1'}]},
  {kind:0,spanId:'11',parentId:'10',name:6,threadName:1,monotonicNs:'2'},
  {kind:4,spanId:'11',parentId:'10',name:6,threadName:1,monotonicNs:'8'}]}}));
 assert.equal(s.spanGeneration(s.spans.get('5:11')!),key(ref));
 assert.deepEqual(s.ancestors(key(ref)),new Set([key(ref),key(r('camera')),key(r('lidar'))]));
});
test('shared zoom keeps the pointer time anchored and enforces bounds',()=>{
 const v={seconds:8,right:20,live:true};const next=zoom(v,.3,-100);
 assert.ok(Math.abs((v.right-v.seconds+.3*v.seconds)-(next.right-next.seconds+.3*next.seconds))<1e-12);
 assert.equal(next.live,false);assert.ok(zoom(v,.5,1e9).seconds<=60);
});
test('browser stores remain bounded',()=>{
 const s=new Store();for(let i=1;i<=2400;i++)s.ingest(frame(i,{step:{generation:r('camera',i),clockDomain:'clock',startNs:String(i)}}));
 assert.equal(s.steps.size,2000);
});
test('paused history survives live patches, retention eviction, and collector restart',()=>{
 const live=new Store(),ref=r('detect');
 live.ingest(frame(1,{clock:{domain:'clock',unixOriginNs:'1000'}}));
 live.ingest(frame(2,{step:{generation:ref,clockDomain:'clock',startNs:'100',inputs:{refs:[r('camera')]}}}));
 live.ingest(frame(3,{activity:{version:1,session:'5',unixOriginNs:'1000',dictionary:['step','detect'],records:[{kind:0,spanId:'10',name:0,threadName:1,monotonicNs:'100'}]}}));
 live.ingest(frame(4,{result:{measurement:'latency',anchor:ref,status:1,durationNs:'10',end:{generation:ref,timestampNs:'110',clockDomain:'clock'}}}));
 const paused=live.snapshot(),original=paused.snapshot();
 live.ingest(frame(5,{step:{generation:ref,clockDomain:'clock',endNs:'120',inputs:{refs:[]}}}));
 live.ingest(frame(6,{activity:{version:1,session:'5',unixOriginNs:'1000',records:[{kind:4,spanId:'10',monotonicNs:'130'}]}}));
 // More than the latency sample cap, then beyond the time-based history window.
 for(let i=7;i<1608;i++)live.ingest(frame(i,{result:{measurement:'latency',anchor:r('detect',i),status:1,durationNs:'10',end:{generation:r('detect',i),timestampNs:String(i),clockDomain:'clock'}}}));
 assert.equal(live.results.length,1500);
 assert.ok(!live.results.some(s=>s.id===paused.results[0].id));
 live.ingest(frame(1608,{step:{generation:r('detect',1608),clockDomain:'clock',endNs:'70000000000'}}));
 assert.equal(live.steps.has(key(ref)),false);assert.equal(live.spans.size,0);assert.equal(live.results.length,0);
 assert.deepEqual(paused,original);
 assert.equal(paused.steps.get(key(ref))!.end,undefined);
 assert.equal(paused.spans.get('5:10')!.end,undefined);
 assert.equal(paused.results.length,1);
 assert.ok(live.latest>paused.latest); // Resume reads the current live store.
 live.ingest(frame(1,{status:{node:'camera',pid:456}},'monitor-2'));
 assert.deepEqual(paused,original);assert.notEqual(live.stream,paused.stream);
});
