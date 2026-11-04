import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Graphs } from '../src/graphs.js';
import { capabilities } from '../src/native.js';
import { releaseConfig } from '../src/release.js';
import { validateApps } from '../src/system-apps.js';

const consoleRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

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

test('watchdog links only selected health outputs and can run alone', async () => {
  // Arrange
  const fragments: Record<string, unknown> = {
    monitor: {nodes:[{id:'monitor',path:'dynamic',outputs:['health']}]},
    recorder: {nodes:[{id:'recorder',path:'dynamic',outputs:['health']}]},
    watchdog: {nodes:[{id:'watchdog',path:'dynamic',outputs:['faults']}]},
  };
  const native = {expandGraph: async (path:string) => Buffer.from(JSON.stringify(fragments[path]))};
  const graphs = new Graphs(native, [{id:'monitor',graph:'monitor'},{id:'recorder',graph:'recorder'},{id:'watchdog',graph:'watchdog',healthInputs:true}], '');
  for (const sources of [[], ['monitor'], ['monitor','recorder']]) {
    // Act
    const graph = JSON.parse((await graphs.build([...sources,'watchdog'])).toString());
    // Assert
    const inputs = graph.nodes.find((node:any) => node.id === 'watchdog').inputs;
    assert.deepEqual(Object.values(inputs).map((input:any) => input.source), sources.map(id => `${id}/health`));
    assert.ok(Object.values(inputs).every((input:any) => input.queue_policy === 'drop_oldest'));
  }
  // Act
  const graph = JSON.parse((await graphs.build(['monitor'])).toString());
  // Assert
  assert.deepEqual(graph.nodes.map((node:any) => node.id), ['monitor']);
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

test('user-messages example merges its producer with the profile recorder', async () => {
  // Arrange
  const example = join(consoleRoot, 'examples/user-messages');
  const template = capabilities.parseConfig(await readFile(join(example, 'console.pbtxt'), 'utf8'));
  const config = releaseConfig(template, example, consoleRoot, '/state');
  validateApps(config.systemApps);
  const graphs = new Graphs(capabilities, config.systemApps, config.dora!.graph!);
  // Act
  const graph = JSON.parse((await graphs.build(['data-recorder'], [], {'data-recorder':'user-messages'})).toString());
  // Assert
  assert.deepEqual(graph.nodes.map((node:any) => node.id), ['user', 'data-recorder']);
  const recorder = graph.nodes.find((node:any) => node.id === 'data-recorder');
  assert.equal(recorder.inputs.data.source, 'user/messages');
  assert.match(await readFile(join(example, 'recorder.pbtxt'), 'utf8'), /input: "data"/);
});
