import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { Graphs } from '../src/graphs.js';

test('selection composes approved nodes and fresh reads detect graph changes', async () => {
  // Arrange
  const fragments = new Map([['workload',{nodes:[{id:'worker',path:'dynamic'}]}],['recorder',{nodes:[{id:'recorder',path:'dynamic',inputs:{data:'worker/data'}}]}]]);
  const reads: string[] = [];
  const graphs = new Graphs({expandGraph: async path => { reads.push(path); return Buffer.from(JSON.stringify(fragments.get(path))); }}, [{id:'recording',graph:'recorder'}], 'workload');
  // Act / Assert
  assert.deepEqual(JSON.parse((await graphs.build([])).toString()).nodes.map((n:any)=>n.id),['worker']);
  const initial = await graphs.build(['recording']);
  assert.deepEqual(JSON.parse(initial.toString()).nodes.map((n:any)=>n.id),['worker','recorder']);
  fragments.set('workload',{nodes:[{id:'worker',path:'dynamic',outputs:['changed']}]} as any);
  assert.notEqual((await graphs.build(['recording'])).toString(),initial.toString());
  assert.equal(reads.length,5);
  await assert.rejects(graphs.build(['unknown']));
  fragments.set('recorder',{nodes:[{id:'worker',path:'dynamic'}]});
  await assert.rejects(graphs.build(['recording']));
});

test('recorder profiles select a complete native graph for arbitrary user outputs', async () => {
  // Arrange
  const fragments: Record<string, unknown> = {
    user: {nodes:[{id:'user',path:'dynamic',outputs:['messages']}]},
    cpu: {nodes:[{id:'recorder',path:'dynamic',inputs:{data:'cpu/snapshots'}}]},
    custom: {nodes:[{id:'recorder',path:'dynamic',inputs:{data:'user/messages'}}]},
  };
  const graphs = new Graphs({expandGraph: async (path:string) => Buffer.from(JSON.stringify(fragments[path]))},
    [{id:'recorder',graph:'cpu',launchProfiles:[{id:'custom',graph:'custom'}]}], 'user');
  // Act
  const graph = JSON.parse((await graphs.build(['recorder'], [], {recorder:'custom'})).toString());
  // Assert
  assert.equal(graph.nodes.find((node:any) => node.id === 'recorder').inputs.data, 'user/messages');
});
