import { doramcap } from './generated/demo.js';
export const Envelope = doramcap.dataflow_tracing.v1.Envelope;
type Ref = doramcap.provenance.v1.IGenerationRef;
type Integer = number | { toString(): string } | null | undefined;
export const integer = (n: Integer) => n?.toString() ?? '0';
export const big = (n: Integer) => BigInt(integer(n));
export const key = (r: Omit<Ref,'sequence'> & {sequence?:Integer}) => JSON.stringify([r.producer,r.epoch,r.step,r.generation,integer(r.sequence)]);
export interface Step { id:string; ref:Ref; name:string; domain:string; start?:bigint; end?:bigint; inputs:string[]; }
export interface Span { id:string; parent:string; name:string; lane:string; session:string; start?:bigint; end?:bigint; fields:Record<string,string>; }
export interface Sample { id:string; name:string; anchor:string; start?:string; end?:string; time?:bigint; clock?:string; endNs?:bigint; duration?:bigint; status:number; reason:number; }
export interface Viewport { seconds:number; right:number; live:boolean; }
export function zoom(view:Viewport, fraction:number, delta:number):Viewport {
 const f=Math.max(0,Math.min(1,fraction));
 const seconds=Math.max(.1,Math.min(60,view.seconds*Math.exp(Math.max(-1000,Math.min(1000,delta))*.002)));
 const anchor=view.right-view.seconds+f*view.seconds;
 return {seconds,right:Math.max(seconds,anchor+(1-f)*seconds),live:false};
}
export class Store {
 stream=''; last=0n; gaps=0; origin?:bigint; latest=0;
 steps=new Map<string,Step>(); spans=new Map<string,Span>(); results:Sample[]=[];
 clocks=new Map<string,bigint>(); nodes=new Map<string,number>(); received=0;
 // Copy the bounded history so live patches, eviction, and restarts cannot alter a paused view.
 snapshot():Store {
  const copy=new Store();
  Object.assign(copy,{stream:this.stream,last:this.last,gaps:this.gaps,origin:this.origin,latest:this.latest,received:this.received});
  copy.steps=new Map([...this.steps].map(([id,s])=>[id,{...s,ref:{...s.ref},inputs:[...s.inputs]}]));
  copy.spans=new Map([...this.spans].map(([id,s])=>[id,{...s,fields:{...s.fields}}]));
  copy.results=this.results.map(r=>({...r}));
  copy.clocks=new Map(this.clocks);copy.nodes=new Map(this.nodes);
  return copy;
 }
 reset(stream:string) {this.stream=stream;this.last=0n;this.gaps=0;this.origin=undefined;this.latest=0;this.steps.clear();this.spans.clear();this.results=[];this.clocks.clear();this.nodes.clear();}
 seconds(time?:bigint) {return time===undefined || this.origin===undefined ? undefined : Number(time-this.origin)/1e9;}
 absolute(domain:string,time?:bigint) {const origin=this.clocks.get(domain);return time===undefined||origin===undefined?undefined:origin+time;}
 observe(time?:bigint) {const value=this.seconds(time);if(value!==undefined)this.latest=Math.max(this.latest,value);}
 ingest(bytes:Uint8Array) {
  const e=Envelope.decode(bytes);if(e.version!==1)throw Error('Unsupported envelope');
  if(e.streamId && e.streamId!==this.stream)this.reset(e.streamId);
  if(e.gap)return false; // Sequence jumps count each missing frame once, including after replay.
  const seq=big(e.sequence);if(seq<=this.last)return false;
  if(seq>this.last+1n)this.gaps+=Number(seq-this.last-1n);
  this.last=seq;this.received++;
  if(e.clock){const c=e.clock;const origin=big(c.unixOriginNs);this.origin??=origin;this.clocks.set(c.domain!,origin);}
  if(e.status)this.nodes.set(e.status.node!,e.status.pid!);
  if(e.step?.generation){
   const p=e.step,id=key(p.generation!);const old=this.steps.get(id);
   const s:Step=old??{id,ref:p.generation!,name:p.generation!.step!,domain:p.clockDomain!,inputs:[]};
   if(p.startNs!=null)s.start=big(p.startNs);if(p.endNs!=null)s.end=big(p.endNs);
   if(p.inputs)s.inputs=(p.inputs.refs??[]).map(key);
   this.steps.set(id,s);this.observe(this.absolute(s.domain,s.end??s.start));
  }
  if(e.activity){
   const b=e.activity;if(b.version!==1)throw Error('Unsupported activity batch');
   const session=integer(b.session), origin=big(b.unixOriginNs);this.origin??=origin;
   this.clocks.set('activity/'+BigInt(session).toString(16).padStart(16,'0'),origin);
   const name=(n:number|null|undefined)=>b.dictionary?.[n??0]??'?';
   const decoder=new TextDecoder();
   for(const r of b.records??[]){
    const id=`${session}:${integer(r.spanId)}`,time=origin+big(r.monotonicNs);
    const s=this.spans.get(id)??{id,parent:`${session}:${integer(r.parentId)}`,name:name(r.name),lane:name(r.threadName),session,fields:{}} as Span;
    if(r.kind===0)s.start=time;if(r.kind===4)s.end=time;
    for(const f of r.fields??[]){s.fields[name(f.name)]=f.kind===4?decoder.decode(b.strings?.subarray(f.offset??0,(f.offset??0)+(f.length??0))):integer(f.bits);}
    if(r.kind===0||r.kind===2||r.kind===4)this.spans.set(id,s);
    this.observe(time);
   }
  }
  if(e.result){const r=e.result;const sample:Sample={id:`${r.measurement}/${key(r.anchor!)}`,name:r.measurement!,anchor:key(r.anchor!),
   start:r.start?.generation?key(r.start.generation):undefined,end:r.end?.generation?key(r.end.generation):undefined,
   clock:r.end?.clockDomain??undefined,endNs:r.end?.timestampNs!=null?big(r.end.timestampNs):undefined,
   time:r.end?.timestampNs!=null?this.absolute(r.end.clockDomain!,big(r.end.timestampNs)):undefined,
   duration:r.durationNs!=null?big(r.durationNs):undefined,status:r.status??0,reason:r.reason??0};
   this.results.push(sample);this.observe(sample.time);
  }
  for(const r of this.results){if(r.time===undefined&&r.clock)r.time=this.absolute(r.clock,r.endNs);this.observe(r.time);}
  this.prune();return true;
 }
 prune(){
  const cutoff=this.latest-65;
  for(const [id,s] of this.steps){const t=this.seconds(this.absolute(s.domain,s.end??s.start));if(t!==undefined&&t<cutoff)this.steps.delete(id);}
  for(const [id,s] of this.spans){const t=this.seconds(s.end??s.start);if(t!==undefined&&t<cutoff)this.spans.delete(id);}
  while(this.steps.size>2000)this.steps.delete(this.steps.keys().next().value!);
  while(this.spans.size>6000)this.spans.delete(this.spans.keys().next().value!);
  this.results=this.results.filter(r=>r.time===undefined||(this.seconds(r.time)??0)>=cutoff).slice(-1500);
  while(this.clocks.size>16)this.clocks.delete(this.clocks.keys().next().value!);
 }
 spanGeneration(span:Span):string|undefined {
  const seen=new Set<string>();let current:Span|undefined=span;
  while(current && !seen.has(current.id)){
   seen.add(current.id);const f=current.fields;
   if(f.producer&&f.epoch&&f.step&&f.generation)return key({producer:f.producer,epoch:f.epoch,step:f.step,generation:f.generation,sequence:f.sequence??f.generation});
   current=this.spans.get(current.parent);
  }
 }
 ancestors(id:string):Set<string>{const found=new Set<string>(),stack=[id];while(stack.length){const v=stack.pop()!;if(found.has(v))continue;found.add(v);stack.push(...(this.steps.get(v)?.inputs??[]));}return found;}
}
