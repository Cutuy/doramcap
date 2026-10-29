import {decodeSnapshot,encodeConfig,decodeReply} from './protocol.mjs';
import {merge, groups, eventInfo, eventTypes, markerTracks} from './model.mjs';
const $ = id => document.getElementById(id);
const esc = value => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
let state = {run:'',cursor:0,events:[]}, paused=false, selected='', selectedEvent='', frozen=null;
const displayed = () => paused && frozen ? frozen : state;
function filtered(view) { return view.events.filter(r => (!$('slot').value || String(r.fields.slot)===$('slot').value) && (!$('process').value || String(r.pid)===$('process').value)).sort((a,b)=>a.time_ms-b.time_ms || String(a.id).localeCompare(String(b.id))); }
function options(id, values) { const el=$(id), value=el.value; el.innerHTML='<option value="">All</option>'+values.map(([v,label])=>`<option value="${esc(v)}">${esc(label)}</option>`).join(''); el.value=value; }
function render() {
  $('status').textContent=state.state + (paused ? ' · view paused':'');
  $('run').disabled=state.state==='running';
  const view=displayed();
  const c=view.config;
  $('run-context').textContent=view.run?`Run ${view.run.slice(0,8)} · ${c.frames} frames · ${c.slots} slots · ${c.consumers} consumers · ${c.bytes} bytes · ${c.failure || 'normal'} · CPU checksum ${c.checksum?'enabled':'disabled'}`:'No experiment yet';
  options('slot', [...new Set(view.events.map(r=>r.fields.slot).filter(s=>s!==undefined))].sort((a,b)=>a-b).map(s=>[s,`Slot ${s}`]));
  options('process',[...new Map(view.events.map(r=>[r.pid,[r.pid,`${r.role} · ${r.pid}`]])).values()]);
  const stats=view.events.findLast(r=>r.fields.event==='stats')?.fields || {};
  $('stats').innerHTML=[['Published',stats.published],['Published pending',stats.pending],['Reclaimed',stats.reclaimed],['Pool full attempts',stats.full],['Completion deferrals',stats.completion_blocks],['Trace records',view.events.length]].map(([k,v])=>`<div class="stat"><strong>${v??'—'}</strong><span>${k}</span></div>`).join('');
  const losses=view.events.some(r=>Object.values(r.losses).some(n=>n>0));
  const batches=new Map();let gaps=false;
  for(const r of view.events){const last=batches.get(r.session);if(last!==undefined&&r.batch>last+1)gaps=true;batches.set(r.session,r.batch);}
  $('warning').textContent=[view.truncated?'History truncated. Download full MCAP / Protobuf for this run.':'',losses?'Activity capture reports loss; observations may be incomplete.':'',gaps?'Activity batch sequence gap detected.':''].filter(Boolean).join(' ');
  const rows=filtered(view), gs=groups(rows).slice(-80), origin=view.events.length?Math.min(...view.events.map(r=>r.time_ms)):0;
  const all=gs.flatMap(g=>g.rows), min=all.length?Math.min(...all.map(r=>r.time_ms)):0, max=all.length?Math.max(...all.map(r=>r.time_ms)):1;
  const x=t=>140+((t-min)/Math.max(1,max-min))*1050;
  let height=28;
  const layouts=gs.map(g=>{
    const markers=markerTracks(g.rows,x);
    const top=height;
    height+=Math.max(32,(1+Math.max(0,...markers.map(m=>m.track)))*20+12);
    return {g,markers,top};
  });
  const focused=document.activeElement?.dataset?.eventId;
  $('timeline').innerHTML=`<svg viewBox="0 0 1240 ${Math.max(80,height+40)}" role="group" aria-label="Slot generation activity timeline">`+layouts.map(({g,markers,top})=>{
    return `<g data-key="${g.slot}:${g.generation}"><text x="0" y="${top+4}" fill="#b3c7e2" font-size="13">Slot ${g.slot} / g${g.generation}</text><line x1="${x(g.rows[0].time_ms)}" x2="${x(g.rows.at(-1).time_ms)}" y1="${top}" y2="${top}" stroke="#3a526d" stroke-width="3"/>`+markers.map(({row:r,x:cx,track})=>{
      const info=eventInfo(r), cy=top+track*20, picked=String(r.id)===selectedEvent;
      const label=`${info.label} · ${r.role} PID ${r.pid} · slot ${g.slot}, generation ${g.generation} · ${(r.time_ms-origin).toFixed(3)} ms`;
      return `<g data-event-id="${esc(r.id)}" role="button" tabindex="0" aria-pressed="${picked}" aria-label="${esc(label)}" class="event-marker"><circle cx="${cx}" cy="${cy}" r="9" fill="transparent"/><circle cx="${cx}" cy="${cy}" r="${picked?7:5}" fill="${info.color}" stroke="${picked?'#fff':info.color}" stroke-width="2"/><title>${esc(label)} — ${esc(info.meaning)}</title></g>`;
    }).join('')+'</g>';
  }).join('')+`<text x="140" y="${height+20}" fill="#9baec7" font-size="12">${(min-origin).toFixed(2)} ms</text><text x="1190" y="${height+20}" text-anchor="end" fill="#9baec7" font-size="12">${(max-origin).toFixed(2)} ms</text></svg>`;
  if(focused) [...$('timeline').querySelectorAll('[data-event-id]')].find(e=>e.dataset.eventId===focused)?.focus({preventScroll:true});
  $('generations').innerHTML=gs.toReversed().map(g=>`<tr data-key="${g.slot}:${g.generation}" class="${selected===`${g.slot}:${g.generation}`?'selected':''}"><td>${g.slot} / ${g.generation}</td><td>${g.rows.some(r=>r.fields.event==='reclaimed')?'Reclaimed':'Observed / outstanding'}</td><td>${g.checks.length?g.checks.map(r=>`<span class="${r.fields.matches?'good':'bad'}">${r.pid}: ${r.fields.checksum} ${r.fields.matches?'✓':'✗'}</span>`).join('<br>'):'Not sampled'}</td></tr>`).join('');
  const picked=view.events.find(r=>String(r.id)===selectedEvent);
  const g=groups(view.events).find(g=>`${g.slot}:${g.generation}`===selected);
  if(picked) {
    const info=eventInfo(picked);
    $('detail-title').textContent=info.label;
    const values=[['Process',`${picked.role} · PID ${picked.pid}`],['Slot / generation',picked.fields.slot===undefined?'—':`${picked.fields.slot} / ${picked.fields.generation}`],['Elapsed',`${(picked.time_ms-origin).toFixed(3)} ms`],['Host timestamp',`${picked.time_ms.toFixed(3)} ms since Unix epoch`],['Session',picked.session],['Batch / record',`${picked.batch} / ${picked.id}`],...Object.entries(picked.fields)];
    $('details').innerHTML=`<p>${esc(info.meaning)}</p><table>${values.map(([key,value])=>`<tr><th>${esc(key)}</th><td>${esc(value)}</td></tr>`).join('')}</table>`;
  } else {
    $('detail-title').textContent=g?`Slot ${g.slot} · generation ${g.generation}`:'Select an event';
    $('details').innerHTML=g?'<p>Choose an individual event below or a timeline circle.</p><table>'+g.rows.map(r=>`<tr data-event-id="${esc(r.id)}" tabindex="0" role="button"><td>${(r.time_ms-origin).toFixed(3)} ms</td><td>${r.pid}</td><td>${esc(eventInfo(r).label)}</td></tr>`).join('')+'</table>':selectedEvent?'Selected event is no longer retained.':'Click a circle to inspect that event instance. Overlapping marks use separate vertical tracks at the same time coordinate.';
  }
  $('records').innerHTML=rows.slice(-200).toReversed().map(r=>`<tr data-event-id="${esc(r.id)}" tabindex="0" role="button" class="${String(r.id)===selectedEvent?'selected':''}"><td>${(r.time_ms-origin).toFixed(2)} ms</td><td>${esc(r.role)} ${r.pid}</td><td>${esc(r.fields.event||r.name)}</td><td>${esc(JSON.stringify(r.fields))}</td></tr>`).join('');
  $('artifacts').innerHTML=view.artifacts.map(name=>`<a href="./artifact/${encodeURIComponent(name)}" download>${esc(name)}</a>`).join('');
  $('log').textContent=view.log || 'Waiting for process summaries; activity records stream live above.';
}
$('controls').onsubmit=async e=>{e.preventDefault();$('error').textContent='';const data=new FormData(e.target);const config=Object.fromEntries(['frames','slots','consumers','bytes'].map(k=>[k,Number(data.get(k))]));config.failure=data.get('failure');config.checksum=data.has('checksum');try{const r=await fetch(new URL('api/run',document.baseURI),{method:'POST',headers:{'Content-Type':'application/x-protobuf'},body:encodeConfig(config)});if(!r.ok)throw Error(decodeReply(new Uint8Array(await r.arrayBuffer())).error);paused=false;frozen=null;selected='';selectedEvent='';$('pause').textContent='Pause view';}catch(e){$('error').textContent=e.message;}};
$('pause').onclick=()=>{if(paused&&frozen?.run!==state.run){selected='';selectedEvent='';}paused=!paused;frozen=paused?state:null;$('pause').textContent=paused?'Resume view':'Pause view';render();};
$('slot').onchange=$('process').onchange=render;
function selectTarget(target) {
  const marker=target.closest('[data-event-id]');
  if(marker) {
    const row=displayed().events.find(r=>String(r.id)===marker.dataset.eventId);
    if(!row)return;
    selectedEvent=String(row.id);
    selected=row.fields.slot===undefined?'':`${row.fields.slot}:${row.fields.generation}`;
    render();
    return;
  }
  const generation=target.closest('[data-key]');
  if(generation) {selected=generation.dataset.key;selectedEvent='';render();}
}
document.addEventListener('click',e=>selectTarget(e.target));
document.addEventListener('keydown',e=>{
  if((e.key==='Enter'||e.key===' ') && e.target.closest('[data-event-id]')) {
    e.preventDefault();selectTarget(e.target);
  }
});
$('legend').innerHTML=Object.values(eventTypes).map(v=>`<span><i style="background:${v.color}"></i>${esc(v.label)}</span>`).join('');
$('download').onclick=async()=>{
  try {
    const response=await fetch(new URL('api/state',document.baseURI));
    if(!response.ok)throw Error(`HTTP ${response.status}`);
    const url=URL.createObjectURL(new Blob([await response.arrayBuffer()],{type:'application/x-protobuf'}));
    const a=document.createElement('a');a.href=url;a.download=`cuda-shm-${state.run}.pb`;a.click();
    setTimeout(()=>URL.revokeObjectURL(url),1000);
  } catch(e) { $('error').textContent=e.message; }
};
async function poll(){try{const r=await fetch(new URL(`api/state?since=${state.cursor}&run=${state.run}&state=${state.state||''}`,document.baseURI));if(r.status!==204 && r.status!==304){if(!r.ok)throw Error(`HTTP ${r.status}`);const next=decodeSnapshot(new Uint8Array(await r.arrayBuffer()));if(next.run!==state.run&&!paused){selected='';selectedEvent='';}state=merge(state,next);if(!paused)render();}}catch(e){$('status').textContent=`Disconnected: ${e.message}`;}setTimeout(poll,150);}poll();
