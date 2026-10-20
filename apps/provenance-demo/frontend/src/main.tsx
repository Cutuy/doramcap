import React, {useEffect,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {Store,zoom,type Viewport,type Sample} from './model';
import './style.css';
import {doramcap} from './generated/demo.js';
const liveStore=new Store();
const names=['camera','lidar','detect','control'];
const colors:Record<string,string>={camera:'#5bd6bc',lidar:'#72a9ff',detect:'#ba9df5',control:'#f5ba6a'};
const metrics=['camera_to_detection','camera_to_control','lidar_age_at_control','detection_duration'];
function App(){
 const [,refresh]=useState(0),[connection,setConnection]=useState('Connecting'),[view,setViewport]=useState<Viewport>({seconds:8,right:8,live:true});
 const [paused,setPaused]=useState<Store>();
 const store=paused??liveStore;
 const setView=(next:Viewport)=>{
  setPaused(current=>next.live?undefined:current??liveStore.snapshot());
  setViewport(next);
 };
 const [filter,setFilter]=useState('all'),[selected,setSelected]=useState<string>(),[sample,setSample]=useState<Sample>(),[summary,setSummary]=useState<any>({});
 const [hover,setHover]=useState<string>();const drag=useRef<{x:number,right:number,width:number}|undefined>(undefined);
 useEffect(()=>{
  let stopped=false,ws:WebSocket,timer:ReturnType<typeof setTimeout>;
  const connect=()=>{ws=new WebSocket(location.origin.replace('http','ws')+'/stream');ws.binaryType='arraybuffer';
   ws.onopen=()=>setConnection('Live');ws.onerror=()=>setConnection('Reconnecting');
   ws.onmessage=e=>{try{if(e.data instanceof ArrayBuffer)liveStore.ingest(new Uint8Array(e.data));}catch(err){setConnection('Decode error');console.error(err);}};
   ws.onclose=()=>{if(!stopped){setConnection('Reconnecting');timer=setTimeout(connect,1000);}};
  };connect();
  const tick=setInterval(()=>refresh(n=>n+1),100);
  const status=setInterval(()=>fetch('/api/summary').then(r=>r.json()).then(setSummary).catch(()=>{}),1000);
  return()=>{stopped=true;clearInterval(tick);clearInterval(status);clearTimeout(timer);ws.close();};
 },[]);
 const right=view.live?Math.max(view.seconds,store.latest+.05):view.right,left=right-view.seconds;
 const range={...view,right};const x=(t:number)=>90+Math.max(-100,Math.min(1100,(t-left)/view.seconds*900));
 const related=selected?store.ancestors(selected):new Set<string>();
 const choose=(id:string,s?:Sample)=>{setSelected(id);setSample(s);};
 const interactions={
  onWheel:(e:React.WheelEvent<SVGSVGElement>)=>{e.preventDefault();const rect=e.currentTarget.getBoundingClientRect();setView(zoom(range,((e.clientX-rect.left)/rect.width*1000-90)/900,e.deltaY));},
  onPointerDown:(e:React.PointerEvent<SVGSVGElement>)=>{if((e.target as Element).getAttribute('data-id'))return;const r=e.currentTarget.getBoundingClientRect();drag.current={x:e.clientX,right,width:r.width*.9};e.currentTarget.setPointerCapture(e.pointerId);setView({...range,live:false});},
  onPointerMove:(e:React.PointerEvent<SVGSVGElement>)=>{if(drag.current)setView({...range,live:false,right:Math.max(view.seconds,drag.current.right-(e.clientX-drag.current.x)/drag.current.width*view.seconds)});},
  onPointerUp:()=>{drag.current=undefined;},
 };
 const axis=(height:number)=><>{Array.from({length:6},(_,i)=>{const t=left+i*view.seconds/5;return <g key={i}><line x1={x(t)} x2={x(t)} y1={25} y2={height-25} className="grid"/><text x={x(t)} y={height-7} textAnchor="middle" className="tick">{t.toFixed(2)}s</text></g>;})}</>;
 const visible=(s:number,e:number)=>e>=left&&s<=right;
 const coords=new Map<string,{x:number,y:number,end:number}>();
 const stepBars=[...store.steps.values()].flatMap(s=>{
  const a=store.seconds(store.absolute(s.domain,s.start)),b=store.seconds(store.absolute(s.domain,s.end));
  if(a===undefined&&b===undefined)return[];const start=a??b!,end=b??Math.max(start,store.latest);
  if(!visible(start,end)||(filter!=='all'&&s.name!==filter))return[];
  const y=55+names.indexOf(s.name)*65;coords.set(s.id,{x:x(start),end:x(end),y:y+10});return[{s,start,end,y}];
 });
 const stepView=<svg data-testid="step-timeline" data-range={`${left}:${right}`} viewBox="0 0 1000 350" {...interactions}>
  {axis(350)}{names.map((n,i)=><text key={n} x="9" y={70+i*65} className="lane">{n}</text>)}
  {stepBars.flatMap(({s})=>s.inputs.map(input=>{const a=coords.get(input),b=coords.get(s.id);return a&&b?<path key={input+s.id} d={`M${a.end} ${a.y} C${a.end+15} ${a.y},${b.x-15} ${b.y},${b.x} ${b.y}`} className={related.has(s.id)?'edge active':'edge'}/>:null;}))}
  {stepBars.map(({s,start,end,y})=><rect key={s.id} data-id={s.id} x={Math.max(90,x(start))} y={y} width={Math.max(3,Math.min(990,x(end))-Math.max(90,x(start)))} height="20" rx="4" fill={colors[s.name]} opacity={!selected||related.has(s.id)?1:.22} stroke={selected===s.id?'white':'none'} strokeWidth="2" onClick={()=>choose(s.id)} onPointerEnter={()=>setHover(`${s.name} #${s.ref.generation} · ${s.end&&s.start?(Number(s.end-s.start)/1e6).toFixed(2)+' ms':'partial'}`)}><title>{s.id}</title></rect>)}
 </svg>;
 const spans=[...store.spans.values()].filter(s=>s.start!==undefined&&(filter==='all'||s.lane===filter));
 const activityView=<svg data-testid="activity-timeline" data-range={`${left}:${right}`} viewBox="0 0 1000 350" {...interactions}>
  {axis(350)}{names.map((n,i)=><text key={n} x="9" y={70+i*65} className="lane">{n}</text>)}
  {spans.map(s=>{const start=store.seconds(s.start)!,end=store.seconds(s.end)??Math.max(start,store.latest);if(!visible(start,end))return null;
   const id=store.spanGeneration(s),y=55+names.indexOf(s.lane)*65+(s.name==='step'?0:24);if(y<40)return null;
   return <rect key={s.id} data-id={s.id} x={Math.max(90,x(start))} y={y} width={Math.max(3,Math.min(990,x(end))-Math.max(90,x(start)))} height={s.name==='step'?20:12} rx="3" fill={colors[s.lane]??'#aaa'} opacity={!selected||(id&&related.has(id))?1:.22} onClick={()=>id&&choose(id)} onPointerEnter={()=>setHover(`${s.lane} / ${s.name} · ${((end-start)*1000).toFixed(2)} ms`)}><title>{s.name} · {s.id}</title></rect>;
  })}
 </svg>;
 const samples=store.results.filter(r=>r.status===1&&r.duration!==undefined&&r.time!==undefined&&visible(store.seconds(r.time)!,store.seconds(r.time)!)&&(filter==='all'||store.steps.get(r.anchor)?.name===filter));
 const maxMs=Math.max(25,...samples.map(r=>Number(r.duration)/1e6))*1.1;
 const latency=<svg data-testid="latency-plot" data-range={`${left}:${right}`} viewBox="0 0 1000 225" {...interactions}>
  {axis(225)}{[0,.5,1].map(f=><g key={f}><text x="10" y={182-f*145} className="tick">{(f*maxMs).toFixed(0)} ms</text><line x1="90" x2="990" y1={177-f*145} y2={177-f*145} className="grid"/></g>)}
  {samples.map(r=><circle key={r.id} data-id={r.id} cx={x(store.seconds(r.time)!)} cy={177-Number(r.duration)/1e6/maxMs*145} r={sample?.id===r.id?6:3.2} fill={Object.values(colors)[metrics.indexOf(r.name)]} opacity={!selected||r.anchor===selected?.toString()?1:.25} onClick={()=>choose(r.anchor,r)} onPointerEnter={()=>setHover(`${r.name} · ${(Number(r.duration)/1e6).toFixed(2)} ms`)}><title>{r.name}: {(Number(r.duration)/1e6).toFixed(2)} ms</title></circle>)}
 </svg>;
 const picked=selected?store.steps.get(selected):undefined;
 return <main><header><div><div className="eyebrow">DORA / LIVE OBSERVABILITY</div><h1>Provenance lab<span className="badge">{connection}</span></h1><p>Four nodes. One pipeline process. Every output has a history.</p></div><div className="architecture"><span>camera + lidar</span><b>→</b><span>detection</span><b>→</b><span>control</span></div></header>
  <section className="stats"><article><small>PIPELINE</small><strong>{store.nodes.size}<em> / 4 nodes</em></strong><span>PID {Object.values(summary.nodes??{})[0] ? (Object.values(summary.nodes)[0] as any).pid : '—'}</span></article><article><small>MEASURED PAIRS</small><strong>{summary.complete??'—'}</strong><span>{summary.incomplete??0} incomplete</span></article><article><small>ACTIVITY SPANS</small><strong>{summary.closed_spans??'—'}</strong><span>shared capture clock</span></article><article><small>STREAM HEALTH</small><strong className={store.gaps?'warning':''}>{store.gaps} <em>gaps</em></strong><span>{summary.capture_losses??0} capture losses · {summary.rejected??0} rejected</span></article></section>
  <nav><div className="controls"><button className={view.live?'active':''} onClick={()=>setView({...range,live:!view.live})}>{view.live?'● Following live':'Resume live'}</button><button onClick={()=>setView(zoom(range,.5,-250))}>＋</button><button onClick={()=>setView(zoom(range,.5,250))}>−</button><select aria-label="Step filter" value={filter} onChange={e=>setFilter(e.target.value)}><option value="all">All steps</option>{names.map(n=><option key={n}>{n}</option>)}</select><span>{view.seconds.toFixed(2)}s window</span></div><span className="hint">Scroll to zoom · drag to pan · select to trace</span></nav>
  <section className="timelines"><article className="panel"><div className="panel-title"><h2>Step provenance</h2><span>generations + dependencies</span></div>{stepView}</article><article className="panel"><div className="panel-title"><h2>Runtime activity</h2><span>step spans + nested work</span></div>{activityView}</article></section>
  <section className="panel latency"><div className="panel-title"><h2>Measured latency</h2><div className="legend">{metrics.map((m,i)=><span key={m}><i style={{background:Object.values(colors)[i]}}/>{m.replaceAll('_',' ')}</span>)}</div></div>{latency}</section>
  {store.results.some(r=>r.status===2)&&<section className="panel incomplete"><h2>Incomplete measurements</h2>{store.results.filter(r=>r.status===2).slice(-5).reverse().map(r=><button key={r.id} onClick={()=>choose(r.anchor,r)}>{r.name.replaceAll('_',' ')} · {doramcap.provenance.v1.Reason[r.reason]?.toLowerCase().replaceAll('_',' ')??'unavailable'}</button>)}</section>}
  <section className="details"><div><small>INSPECTOR</small><h3>{sample?sample.name.replaceAll('_',' '):picked?`${picked.name} / generation ${picked.ref.generation}`:'Select a generation, span, or latency point'}</h3><p>{sample?`${sample.duration===undefined?'Incomplete':(Number(sample.duration)/1e6).toFixed(3)+' ms'} · two exact timestamps`:picked?`${picked.inputs.length} direct inputs · ${related.size} generations in ancestry`:'Selections are linked across both timelines and the latency plot.'}</p></div>{selected&&<div className="references"><code>{selected}</code>{sample&&<><label>Start probe</label><code>{sample.start}</code><label>End probe</label><code>{sample.end}</code></>}<button onClick={()=>{setSelected(undefined);setSample(undefined);}}>Clear selection</button></div>}</section><footer>{hover??'Data travels through real Dora connections. Timeline clocks are aligned at capture.'}<span>Monitor PID {summary.monitor_pid??'—'}</span></footer>
 </main>;
}
createRoot(document.getElementById('root')!).render(<App/>);
