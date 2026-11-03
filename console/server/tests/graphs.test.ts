import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { Graphs, MissingSource } from '../src/graphs.js';

test('selection composes approved nodes and fresh reads detect graph changes', async () => {
  // Arrange
  const fragments = new Map([['workload',{nodes:[{id:'worker',path:'dynamic',outputs:['data']}]}],['recorder',{nodes:[{id:'recorder',path:'dynamic',inputs:{data:'worker/data'}}]}]]);
  const reads: string[] = [];
  const graphs = new Graphs({expandGraph: async path => { reads.push(path); return Buffer.from(JSON.stringify(fragments.get(path))); }}, [{id:'recording',graph:'recorder'}], 'workload');
  // Act / Assert
  assert.deepEqual(JSON.parse((await graphs.build([])).toString()).nodes.map((n:any)=>n.id),['worker']);
  const initial = await graphs.build(['recording']);
  assert.deepEqual(JSON.parse(initial.toString()).nodes.map((n:any)=>n.id),['worker','recorder']);
  fragments.set('workload',{nodes:[{id:'worker',path:'dynamic',outputs:['data','changed']}]} as any);
  assert.notEqual((await graphs.build(['recording'])).toString(),initial.toString());
  assert.equal(reads.length,5);
  await assert.rejects(graphs.build(['unknown']));
  fragments.set('recorder',{nodes:[{id:'worker',path:'dynamic'}]} as any);
  await assert.rejects(graphs.build(['recording']));
});

test('a selected node with a missing source names it, and dora timers need no node', async () => {
  // Arrange
  const fragments = new Map<string, unknown>([
    ['project', {nodes:[{id:'workload',path:'dynamic',inputs:{tick:'dora/timer/millis/20'},outputs:['faults']}]}],
    ['other', {nodes:[{id:'worker',path:'dynamic'}]}],
    ['watchdog', {nodes:[{id:'fault-watchdog',path:'dynamic',inputs:{faults:{source:'workload/faults'}}}]}],
  ]);
  const native = {expandGraph: async (path: string) => Buffer.from(JSON.stringify(fragments.get(path)))};
  const apps = [{id:'watchdog',graph:'watchdog'}];
  // Act / Assert
  await new Graphs(native, apps, '').build(['watchdog'], ['project']);
  await assert.rejects(new Graphs(native, apps, '').build(['watchdog'], ['other']), (error: Error) => error instanceof MissingSource && /fault-watchdog needs Dora output workload\/faults/.test(error.message));
  fragments.set('project', {nodes:[{id:'workload',path:'dynamic',outputs:['other']}]});
  await assert.rejects(new Graphs(native, apps, '').build(['watchdog'], ['project']), MissingSource);
});
