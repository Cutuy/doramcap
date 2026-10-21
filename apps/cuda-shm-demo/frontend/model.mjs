export const limit = 20000;
export function merge(state, response) {
  const reset = state.run !== response.run;
  const rows = (reset ? [] : state.events).concat(response.events).slice(-limit);
  const events = [...new Map(rows.map(r => [r.id, r])).values()];
  return {...response, log: response.log ?? (reset ? '' : state.log), events, truncated: response.oldest > 1 || events.length === limit};
}
export function groups(events) {
  const result = new Map();
  for (const row of [...events].sort((a,b) => a.time_ms-b.time_ms || String(a.id).localeCompare(String(b.id)))) {
    const f = row.fields;
    if (f.slot === undefined || f.generation === undefined) continue;
    const key = `${f.slot}:${f.generation}`;
    if (!result.has(key)) result.set(key, {slot:f.slot, generation:f.generation, rows:[], checks:[]});
    const group = result.get(key);
    group.rows.push(row);
    if (f.event === 'cpu_checksum') group.checks.push(row);
  }
  return [...result.values()];
}

// Meanings describe observed host operations, never inferred GPU execution times.
export const eventTypes = {
  reserved: {color:'#699cff', label:'Slot reserved', meaning:'The producer acquired the exclusive write reservation for this slot generation.'},
  ready_record_queued: {color:'#ffcb7c', label:'Ready record queued', meaning:'cudaEventRecord(Ready) returned after producer writes were queued. GPU writes may still be pending.'},
  ready_wait_queued: {color:'#51d8ed', label:'Ready wait queued', meaning:'The consumer queued cudaStreamWaitEvent(Ready) on its chosen stream. Later reads depend on Ready; this does not mean the GPU has reached the wait.'},
  done_record_queued: {color:'#ba99ff', label:'Done record queued', meaning:'The consumer queued cudaEventRecord(Done) after its reads. The GPU may still be reading.'},
  returned_received: {color:'#ff93c5', label:'Return received', meaning:'The producer accepted this consumer’s Returned message. CUDA completion is checked separately before reuse.'},
  reclaimed: {color:'#65dfb5', label:'Slot reclaimed', meaning:'All consumers returned this generation, Ready and every Done were observed complete, and the local write lease retired. The slot can be reused.'},
  cpu_checksum: {color:'#e0eeff', label:'CPU checksum sampled', meaning:'After waiting for Ready on the CPU, the consumer summed the slot bytes and compared the result with the expected pattern. This wait changes consumer timing.'},
  return_sent: {color:'#8a9bb0', label:'Return sent', meaning:'The consumer sent Returned metadata after queuing Done. Sending the message does not imply GPU completion.'},
};
export function eventInfo(row) {
  return eventTypes[row.fields.event] ?? {color:'#8a9bb0',label:row.fields.event || row.name,meaning:'Host activity record. Inspect its fields for the observed operation.'};
}
// Keep true time coordinates; separate overlapping hit targets vertically.
export function markerTracks(rows, x, gap=20) {
  const ends=[];
  return [...rows].sort((a,b)=>a.time_ms-b.time_ms).map(row=>{
    const position=x(row.time_ms);
    let track=ends.findIndex(end=>position-end>=gap);
    if(track===-1)track=ends.length;
    ends[track]=position;
    return {row,x:position,track};
  });
}
