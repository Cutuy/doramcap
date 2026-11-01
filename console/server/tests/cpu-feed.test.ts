import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { CpuFeeds } from '../src/cpu-feed.js';
import { viewer } from '../src/native.js';

test('CPU feed reuses viewer packets, bounds history and strips host ancestor paths', () => {
  // Arrange
  const feeds = new CpuFeeds({status: async () => Buffer.alloc(0)}, [{appId:'metrics',url:'ws://127.0.0.1:1/stream'}]);
  const base = {runId:'run',phase:'FINISHED',systemApps:{ids:['metrics']}};
  feeds.enrich(base);
  feeds.observe(viewer.Packet.encode(viewer.Packet.fromObject({dictionary:{version:2,entries:[{id:'3',path:'/private/host/worker.scope'}]}})).finish());
  // Act
  for (const sequence of ['1','2','3']) feeds.observe(viewer.Packet.encode(viewer.Packet.fromObject({snapshot:{version:2,snapshot:{sequence,records:[{cgroupId:'3',parentId:'1'}]}}})).finish());
  const status = feeds.enrich(base);
  // Assert
  assert.deepEqual(status.cpuSnapshots.map((s: any) => s.sequence), ['2','3']);
  assert.deepEqual(status.cpuGroupNames, [{id:'3',name:'worker.scope'}]);
  assert.equal(status.monitorReady,true);
  assert.equal(status.cpuCaptureEnabled,true);
  assert.equal(feeds.enrich({...base,systemApps:{ids:[]}}).cpuCaptureEnabled,false);
  assert.equal(feeds.enrich({...base,runId:'next'}).cpuSnapshots.length,0);
  feeds.close();
});


test('restored terminal CPU history survives an idle feed consumer',()=>{
  // Arrange
  const feeds=new CpuFeeds({status:async()=>Buffer.alloc(0)},[{appId:'metrics',url:'ws://127.0.0.1:1/stream'}]);
  const saved={runId:'finished',phase:'FINISHED',systemApps:{ids:['metrics']},cpuSnapshots:[{sequence:'1'}],cpuGroupNames:[{id:'3',name:'worker.scope'}]};
  try {
    // Act
    const restored=feeds.enrich(saved);
    // Assert
    assert.deepEqual(restored.cpuSnapshots,saved.cpuSnapshots);assert.deepEqual(restored.cpuGroupNames,saved.cpuGroupNames);
  }finally{feeds.close();}
});

test('CPU totals come from the selected monitor and expire with its feed', () => {
  // Arrange
  const feeds = new CpuFeeds({status: async () => Buffer.alloc(0)}, [{appId:'metrics',url:'ws://127.0.0.1:1/stream'}]);
  const base = {runId:'run',phase:'FINISHED',systemApps:{ids:['metrics']}};
  const observe = (sequence: number, busyTicks: string, totalTicks: string, sessionId = 'monitor') => feeds.observe(viewer.Packet.encode(viewer.Packet.fromObject({snapshot:{version:2,sessionId,snapshot:{sequence,cpuTotals:{busyTicks,totalTicks,cpuCores:2}}}})).finish());
  try {
    feeds.enrich(base);
    // Act / Assert: warm-up retains availability until two real monitor samples arrive.
    observe(1, '9007199254740993', '9007199254741993');
    assert.equal(feeds.sample('run')?.cpuPercent, null);
    observe(2, '9007199254741493', '9007199254742993');
    assert.deepEqual(feeds.sample('run'), {cpuPercent:100,cpuCores:2});
    assert.equal(feeds.sample('another'), undefined);
    assert.equal(feeds.sample('run', performance.now()+1000)?.cpuPercent, null);
    // Act / Assert: idle zero is real data; a restarted producer needs a new baseline.
    observe(3, '9007199254741493', '9007199254743993');
    assert.equal(feeds.sample('run')?.cpuPercent, 0);
    observe(4, '100', '200', 'restarted');
    assert.equal(feeds.sample('run')?.cpuPercent, null);
    // Act / Assert: unselecting the app disables sampling even with retained packets.
    feeds.enrich({...base,systemApps:{ids:[]}});
    assert.equal(feeds.sample('run'), undefined);
  } finally {feeds.close();}
});
