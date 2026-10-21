import {test} from 'node:test';
import assert from 'node:assert/strict';
import {merge, groups} from '../frontend/model.mjs';
test('reconnect resets run and deduplicates replayed records',()=>{
  // Arrange
  const state={run:'old',events:[{id:1}]};
  const response={run:'new',oldest:1,events:[{id:1,fields:{slot:0,generation:1}},{id:2,fields:{slot:0,generation:2}}]};
  // Act
  const first=merge(state,response), replay=merge(first,response);
  // Assert
  assert.equal(replay.events.length,2);
  assert.equal(replay.run,'new');
});
test('out-of-order process arrivals correlate by slot and generation',()=>{
  // Arrange
  const rows=[{id:1,time_ms:30,pid:2,fields:{event:'done_record_queued',slot:0,generation:42}},
    {id:2,time_ms:10,pid:1,fields:{event:'ready_record_queued',slot:0,generation:42}},
    {id:3,time_ms:40,pid:1,fields:{event:'reserved',slot:0,generation:43}},
    {id:4,time_ms:25,pid:2,fields:{event:'cpu_checksum',slot:0,generation:42,checksum:7,matches:false}}];
  // Act
  const result=groups(rows);
  // Assert
  assert.equal(result.length,2);
  assert.equal(result[0].rows[0].fields.event,'ready_record_queued');
  assert.equal(result[0].checks[0].fields.matches,false);
  assert.equal(result[1].generation,43);
  assert.ok(!result[0].rows.some(r=>r.fields.event==='reclaimed'));
});
test('bounded view exposes history truncation',()=>{
  // Arrange
  const response={run:'a',oldest:2,events:Array.from({length:20010},(_,id)=>({id}))};
  // Act
  const result=merge({run:'a',events:[]},response);
  // Assert
  assert.equal(result.events.length,20000);
  assert.equal(result.truncated,true);
});
test('omitted log retains last value and run reset clears it',()=>{
  // Arrange
  const state={run:'a',events:[],log:'finished'};
  // Act
  const incremental=merge(state,{run:'a',oldest:0,events:[]});
  const reset=merge(state,{run:'b',oldest:0,events:[]});
  // Assert
  assert.equal(incremental.log,'finished');
  assert.equal(reset.log,'');
});
test('overlapping instances remain individually reachable at their actual timestamps',async()=>{
  // Arrange
  const {markerTracks,eventInfo}=await import('../frontend/model.mjs');
  const rows=[{id:'1/0',time_ms:10,pid:11,fields:{event:'done_record_queued'}},
    {id:'2/0',time_ms:10,pid:12,fields:{event:'done_record_queued'}},
    {id:'3/0',time_ms:11,pid:13,fields:{event:'returned_received'}},
    {id:'4/0',time_ms:40,pid:11,fields:{event:'reclaimed'}}];
  // Act
  const markers=markerTracks(rows,t=>t);
  // Assert
  assert.deepEqual(markers.map(m=>m.x),[10,10,11,40]);
  assert.deepEqual(markers.map(m=>m.track),[0,1,2,0]);
  assert.equal(new Set(markers.map(m=>m.row.id)).size,4);
  assert.match(eventInfo(rows[0]).meaning,/may still be reading/);
  assert.match(eventInfo(rows[3]).meaning,/observed complete/);
});
