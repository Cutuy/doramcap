import React,{useState} from 'react';
const categories = [
 ['host_binds','Host directories','Direct bind writes remain on the host. Overlay mounting and write-back are pending.'],
 ['container_layers','Container layers','Docker diff captures paths and change kinds. Containers are removed; contents are not exported.'],
 ['named_volumes','Named volumes','Metadata is captured where accessible. This fixture removes its volumes after collection.'],
] as const;
export function FilesystemPanel({result}:{result:any}) {
 const [category,setCategory]=useState<string>('host_binds'),[query,setQuery]=useState(''),[kind,setKind]=useState('all');
 const summaries=result?.report?.filesystem_changes;
 const summary=summaries?.[category], preview=result?.filesystem_preview?.[category];
 const rows=(preview?.rows??[]).filter((r:any)=>(kind==='all'||r.kind===kind)&&`${r.path} ${r.source}`.toLowerCase().includes(query.toLowerCase()));
 const total=summary?Object.values(summary.counts).reduce((a:number,b)=>a+Number(b),0):0;
 return <section className="panel filesystem"><div className="panelhead"><div><h2>Filesystem changes</h2><p>Metadata tracking · Overlay isolation pending</p></div><span className="badge">NO CONTENT HASHING</span></div>
 <div className="storage-tabs" role="tablist" aria-label="Storage category">{categories.map(([key,label])=><button key={key} role="tab" aria-selected={category===key} onClick={()=>setCategory(key)}><strong>{label}</strong><small>{summaries?Object.entries(summaries[key].counts).map(([k,v])=>`${v} ${k}`).join(' · '):'Awaiting collection'}</small></button>)}</div>
 <div className="filebody"><p>{categories.find(c=>c[0]===category)?.[2]}</p>{!summary?<p className="placeholder">Changes appear after workloads stop and before resources are removed.</p>:<>
 {(summary.warnings??[]).map((warning:string,i:number)=><div className="warning" key={i}>{warning}</div>)}{preview?.error&&<div className="warning">Preview unavailable: {preview.error}</div>}
 <div className="filefilters"><input aria-label="Filter file paths" placeholder="Filter paths or sources…" value={query} onChange={e=>setQuery(e.target.value)}/><select aria-label="Change kind" value={kind} onChange={e=>setKind(e.target.value)}>{['all','added','modified','deleted'].map(k=><option key={k}>{k}</option>)}</select><small>{rows.length} shown · {preview?.rows?.length??0} loaded · {total} total</small></div>
 <div className="filetable"><table><thead><tr><th>Change</th><th>Path / source</th><th>Metadata</th></tr></thead><tbody>{rows.map((r:any,i:number)=><tr key={i}><td><span className={`change ${r.kind}`}>{r.kind}</span></td><td><code>{r.path||'.'}</code><small>{r.source}</small><small>{r.consumers?.join(' · ')}</small></td><td><details><summary>{r.before||r.after?'Before / after':'Path only'}</summary><pre>{JSON.stringify({before:r.before,after:r.after},null,2)}</pre></details></td></tr>)}</tbody></table>{!rows.length&&<p>No matching changes in the retained preview.</p>}</div>
 <p className="manifest">Full metadata manifest: <code>{summary.manifest}</code></p><small>Preview bounded to 100 entries / 32 KiB per category. Coverage warnings above describe incomplete observations.</small></>}</div></section>
}
