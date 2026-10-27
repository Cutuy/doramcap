export type Event = { version:number; sequence:string; session:string; invocation:string; step:string; executor:string; parent?:string; dependencies?:string[]; state:string; elapsedNs:string; durationNs:string; payloadJson?:string; error?:string };
export type Step = { id:string; name:string; session:string; executor:string; parent?:string; dependencies:string[]; state:string; start:number; end?:number; input?:string; output?:string; error?:string; observedStart:boolean };
export function steps(events:Event[], session:string):Step[] {
  const map=new Map<string,Step>();
  for (const e of events.filter(e=>e.session===session)) {
    const start=(Number(e.elapsedNs)-Number(e.durationNs))/1e9;
    const old=map.get(e.invocation);
    const step:Step=old ?? {id:e.invocation,name:e.step.split('::').at(-1)!,session,executor:e.executor,parent:e.parent,dependencies:e.dependencies??[],state:e.state,start,observedStart:false};
    if(e.state==='STARTED') {step.observedStart=true;step.input=e.payloadJson;step.start=start;}
    else {step.end=Number(e.elapsedNs)/1e9;step.state=e.state;step.output=e.payloadJson;step.error=e.error;}
    map.set(e.invocation,step);
  }
  return [...map.values()].sort((a,b)=>a.start-b.start);
}
