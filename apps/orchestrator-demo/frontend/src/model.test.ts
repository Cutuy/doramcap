import {test} from 'node:test';
import assert from 'node:assert/strict';
import {steps, type Event} from './model';
const event=(state:string):Event=>({version:1,sequence:'9007199254740993',session:'a',invocation:'9007199254740995',step:'fixture::Audit',executor:'local',dependencies:['9007199254740994'],state,elapsedNs:'2000000000',durationNs:'1000000000'});
test('terminal-before-start retains outcome and exact IDs',()=>{
  // Arrange
  const end={...event('FAILED'),error:'audit failed'};
  const start={...event('STARTED'),payloadJson:'{"path":"/capture"}'};
  // Act
  const result=steps([end,start,start],'a');
  // Assert
  assert.equal(result.length,1);assert.equal(result[0].state,'FAILED');
  assert.equal(result[0].id,'9007199254740995');assert.equal(result[0].input,start.payloadJson);
  assert.equal(result[0].observedStart,true);
});
test('replay without start is explicit and sessions are isolated',()=>{
  // Arrange
  const e=event('SUCCEEDED');
  // Act
  const result=steps([e,{...e,session:'b'}],'a');
  // Assert
  assert.equal(result.length,1);assert.equal(result[0].observedStart,false);assert.equal(result[0].start,1);
});
