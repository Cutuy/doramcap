import {strict as assert} from 'node:assert';
import {test} from 'node:test';
import React from 'react';
import {renderToStaticMarkup} from 'react-dom/server';
import {doramcap} from './generated/console';
import {FaultPanel,ProcessPanel} from './RunHealth';
import {Health,Kind,LiveModel} from './streams';

const fault = doramcap.fault.v1;
const stream = doramcap.console.v1.LiveStream.create({id:'watchdog_0',appId:'fault-watchdog',kind:Kind.FAULT_PUBLICATION});
const runtime = (containers: object[],error='') => doramcap.compose.v1.RuntimeObservation.fromObject({containers,error});
function reported() {
  const source = new Uint8Array(16).fill(4),epoch = new Uint8Array(16).fill(1);
  const model = new LiveModel();
  model.configure('run',[stream]);
  model.receive(doramcap.console.live.v1.Frame.create({stream:'watchdog_0',sequence:1,payload:fault.AggregationPublication.encode(fault.AggregationPublication.fromObject({snapshot:{epoch,revision:1,sources:[{source,connected:false,conditions:[
    {key:{source,condition:1},definition:{component:'system-monitor',resource:'runtime',code:'sampling'},health:Health.HEALTH_HEALTHY,freshness:fault.Freshness.FRESHNESS_STALE},
    {key:{source,condition:2},definition:{component:'system-monitor',resource:'runtime',code:'publication'},health:Health.HEALTH_ACTIVE,freshness:fault.Freshness.FRESHNESS_FRESH},
    {key:{source,condition:3},definition:{component:'camera',code:'read'},health:Health.HEALTH_RECOVERING,freshness:fault.Freshness.FRESHNESS_UNKNOWN},
    {key:{source,condition:4},definition:{component:'camera',code:'pending'},health:Health.HEALTH_UNKNOWN,freshness:fault.Freshness.FRESHNESS_UNKNOWN},
  ]}]}})).finish()}));
  return model.view().faults!;
}

test('service status shows native running, clean exit and unknown facts independently',()=>{
  // Arrange
  const facts=runtime([{containerId:'coordinator',serviceName:'dora-coordinator',running:true},{containerId:'daemon',serviceName:'dora-daemon',exitCode:0},{containerId:'watchdog',serviceName:'fault-watchdog'}],'event stream ended');
  // Act
  const html=renderToStaticMarkup(<ProcessPanel runtime={facts}/>);
  // Assert
  for(const label of ['dora-coordinator','dora-daemon','fault-watchdog','Running','Exited · 0','Unknown','Observation incomplete']) assert.ok(html.includes(label),label);
  assert.doesNotMatch(html,/Healthy/);
});

test('reported health preserves native names, source identity, health and freshness',()=>{
  // Arrange / Act
  const html=renderToStaticMarkup(<FaultPanel faults={reported()}/>);
  // Assert
  for(const label of ['04'.repeat(16),'system-monitor','camera','Healthy','Active','Recovering','Unknown','Stale','Fresh','Disconnected','Last report received','1 active','1 recovering']) assert.ok(html.includes(label),label);
  assert.doesNotMatch(html,/cpu-monitor|data-recorder/);
});

test('stopped reporters and a lost live connection mark retained healthy data as the last report',()=>{
  // Arrange
  const facts=runtime([{containerId:'watchdog',serviceName:'fault-watchdog',exitCode:137},{containerId:'bridge',serviceName:'live-bridge',exitCode:0}]);
  // Act
  const html=renderToStaticMarkup(<FaultPanel faults={reported()} runtime={facts} streams={[stream]} disconnected/>);
  // Assert
  for(const label of ['Connection lost','fault-watchdog exited · 137','live-bridge exited · 0','Last report received','Healthy']) assert.ok(html.includes(label),label);
});

test('unselected and not-yet-received watchdog reports never imply empty healthy success',()=>{
  // Arrange
  const model=new LiveModel();
  model.configure('run',[stream]);
  // Act
  const absent=renderToStaticMarkup(<FaultPanel/>);
  const waiting=renderToStaticMarkup(<FaultPanel faults={model.view().faults}/>);
  const missing=renderToStaticMarkup(<ProcessPanel/>);
  // Assert
  assert.match(absent,/No watchdog stream selected/);
  assert.match(waiting,/Waiting for complete snapshot/);
  assert.match(missing,/No Docker state observed/);
  assert.doesNotMatch(waiting,/No active faults|No reported sources|0 active|Healthy/);
});
