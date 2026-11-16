import {strict as assert} from 'node:assert';
import {test} from 'node:test';
import React from 'react';
import {renderToStaticMarkup} from 'react-dom/server';
import {doramcap} from './generated/console';
import {FaultPanel,ProcessPanel} from './RunHealth';
import { RunMetrics } from './RunMetrics';
import {Health,Kind,LiveModel} from './streams';

const fault = doramcap.fault.v1;
const stream = doramcap.console.v1.LiveStream.create({id:'watchdog_0',appId:'fault-watchdog',kind:Kind.FAULT_PUBLICATION});
const runtime = (containers: object[],error='') => doramcap.compose.v1.RuntimeObservation.fromObject({containers,error});
function reported() {
  const source = new Uint8Array(16).fill(4),epoch = new Uint8Array(16).fill(1);
  const model = new LiveModel();
  model.configure('run',[stream]);
  model.receive(doramcap.console.live.v1.Frame.fromObject({stream:'watchdog_0',sequence:1,receivedUnixNs:'1700000000123456789',payload:fault.AggregationPublication.encode(fault.AggregationPublication.fromObject({sourceApps:[{source,app:'cpu-monitor'}],snapshot:{epoch,revision:1,sources:[{source,connected:false,conditions:[
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
  for(const label of ['cpu-monitor','04'.repeat(16),'system-monitor','camera','Healthy','Active','Recovering','Unknown','Stale','Fresh','Disconnected','Last report received','1 active','1 recovering']) assert.ok(html.includes(label),label);
  assert.match(html, /dateTime="2023-11-14T22:13:20.123Z"/);
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

test('unattributed sources keep their identities and native condition names visible', () => {
  // Arrange
  const faults = reported();
  faults.reportedSources[0].app = undefined;
  faults.receivedAtMs = undefined;
  // Act
  const html = renderToStaticMarkup(<FaultPanel faults={faults}/>);
  // Assert
  for (const label of ['Unknown app', '04'.repeat(16), 'system-monitor', 'Last report time unavailable', '1 active', '1 recovering']) assert.ok(html.includes(label), label);
});


test('watchdog health stays visible without Docker observations in a systemd run', () => {
  // Arrange
  const live = { faults: reported(), gaps: 0 };
  // Act
  const native = renderToStaticMarkup(<RunMetrics showHealth live={live}/>);
  const compose = renderToStaticMarkup(<RunMetrics showHealth live={live} runtime={runtime([])}/>);
  // Assert
  assert.match(native, /Reported health/);
  assert.match(native, /cpu-monitor/);
  assert.doesNotMatch(native, /Service status|Docker/);
  assert.match(compose, /Service status/);
  assert.match(compose, /No Docker state observed/);
});
