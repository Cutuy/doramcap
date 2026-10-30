import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { ActivityTimeline, EventDetails } from './ActivityTimeline';
import { RunActivity } from './RunActivity';
import { RunMetrics } from './RunMetrics';
import { Result } from './Result';
import { SystemApps } from './SystemApps';
import { cpuRecordingEnabled } from './api';
import { doramcap } from './generated/console';
import { PageFrame } from './ui/PageFrame';
import type { Snapshot, Step } from './api';

const step: Step = { session: 'session', executor: 'local', invocation: '9007199254740993', step: 'module::LaunchProjects', state: 'FAILED', durationNs: '100000000', elapsedNs: '1200000000', parent: '9007199254740994', dependencies: ['9007199254740995'], sequence: '9007199254740996', error: 'step failed' };
test('event inspector preserves precise identity, timing, relationships and safe failure information', () => {
  // Arrange / Act
  const html = renderToStaticMarkup(<EventDetails step={step} now={3}/>);
  // Assert
  for (const id of ['9007199254740993', '9007199254740994', '9007199254740995', '9007199254740996']) assert.match(html, new RegExp(id));
  assert.match(html, /0.100 s/);
  assert.match(html, /1.100 s/);
  assert.match(html, /step failed/);
  assert.doesNotMatch(html, /payloads are excluded|private host evidence/);
});
test('timeline events are keyboard buttons and completed view supports elapsed-time browsing', () => {
  // Arrange / Act
  const html = renderToStaticMarkup(<ActivityTimeline steps={[step]} now={2} live={false}/>);
  // Assert
  assert.match(html, /role="button" tabindex="0"/);
  assert.match(html, /Elapsed time/);
  assert.doesNotMatch(html, /Inspect retained event|timeline-event-picker/);
  assert.doesNotMatch(html, /Retained workflow events|Select an event|Scroll the timeline/);
  assert.doesNotMatch(html, /Following live/);
});
test('completed result exposes run activity even when admission is blocked', () => {
  // Arrange
  const snapshot = { phase: 'BLOCKED', runId: 'a'.repeat(32), outcome: null, cpuAvailable: false, reportAvailable: false } as Snapshot;
  // Act
  const html = renderToStaticMarkup(<Result snapshot={snapshot} newRun={() => {}} viewRun={() => {}}/>);
  const rail = renderToStaticMarkup(<PageFrame phase="Result" onNavigate={() => {}}>Result</PageFrame>);
  // Assert
  assert.match(html, /View run activity/);
  assert.match(html, /disabled=""[^>]*>New run/);
  assert.match(rail, /<button[^>]*><small>03<\/small>Run<\/button>/);
});
test('historical run freezes elapsed time and unavailable metrics remain distinct from zero', () => {
  // Arrange
  const snapshot = { phase: 'FINISHED', runId: 'run', steps: [step], sequence: '1', truncatedSteps: '0' } as Snapshot;
  // Act
  const historical = renderToStaticMarkup(<RunActivity snapshot={snapshot} stopping={false} disconnected={false} historical/>);
  const absent = renderToStaticMarkup(<RunMetrics disconnected incomplete/>);
  const zero = renderToStaticMarkup(<RunMetrics samples={[{elapsedNs:'1000000000',recordingBytes:'0',writeBytesPerSecond:0,cpuPercent:0,cpuCores:8}]} historical/>);
  // Assert
  assert.match(historical, /Completed run/);
  assert.doesNotMatch(historical, /Retained view|512 invocations|1200 metric samples/);
  assert.match(absent, /Unavailable/);
  assert.match(absent, /Connection lost/);
  assert.match(absent, /incomplete/);
  assert.match(zero, /0 B/);
  assert.match(zero, /0.0%/);
  assert.match(zero, /8 cores/);
  assert.match(zero, /Top CPU consumers/);
  assert.match(zero, /Capture health/);
  assert.doesNotMatch(zero, /Inclusive ancestors|recorder boundary/);
});

test('completed runs without retained activity do not imply that events will arrive', () => {
  // Arrange
  const snapshot: Snapshot = { phase: 'FINISHED', runId: 'old-run', steps: [], sequence: '0', truncatedSteps: '0', lostEvents: '0', overflowSteps: '0', progressIncomplete: false, outcome: null, cpuAvailable: false, reportAvailable: false, monitorReady: false };
  // Act
  const html = renderToStaticMarkup(<RunActivity snapshot={snapshot} historical stopping={false} disconnected={false}/>);
  // Assert
  assert.match(html, /No workflow steps available/);
  assert.match(html, /No events available/);
  assert.doesNotMatch(html, /Waiting for workflow/);
});


test('system app choices reflect selection and disabled recording stays distinct from failure', () => {
  // Arrange
  const apps = [{ id: 'cpu-monitor', label: 'CPU recording', selected: true }];
  const snapshot: Snapshot = { phase: 'FINISHED', runId: 'a'.repeat(32), steps: [], sequence: '0', lostEvents: '0', truncatedSteps: '0', overflowSteps: '0', progressIncomplete: false, cpuAvailable: false, reportAvailable: true, monitorReady: false, systemApps: { ids: [] }, outcome: { execution: 'Completed', recordingFinalized: null, boundaryClean: true } };
  // Act
  const enabled = renderToStaticMarkup(<SystemApps apps={apps} busy={false} toggle={() => {}}/>);
  const checking = renderToStaticMarkup(<SystemApps apps={[{...apps[0],selected:false}]} busy toggle={() => {}}/>);
  const result = renderToStaticMarkup(<Result snapshot={snapshot} newRun={() => {}}/>);
  // Assert
  assert.match(enabled, /type="checkbox" checked=""/);
  assert.match(enabled, /CPU recording/);
  assert.match(checking, /fieldset disabled=""/);
  assert.doesNotMatch(checking, /checked=""/);
  assert.match(result, /Disabled/);
  assert.match(result, /CPU recording disabled/);
  assert.doesNotMatch(result, /Download CPU recording/);
  assert.equal(cpuRecordingEnabled(snapshot), false);
  assert.equal(cpuRecordingEnabled({ systemApps: { ids: ['cpu-monitor'] } }), true);
  assert.equal(cpuRecordingEnabled({}), true);
});

test('CPU consumers show reported cgroup names with an ID tooltip and numeric fallback', () => {
  // Arrange
  const cpuSnapshots = [1,2].map(sequence=>doramcap.telemetry.v1.Snapshot.fromObject({sequence,monotonicEndNs:sequence*100000000,records:[{cgroupId:'9007199254740993',parentId:0,cpu:0,flags:1,runtimeNs:sequence*10000000}]}));
  // Act
  const named = renderToStaticMarkup(<RunMetrics cpuSnapshots={cpuSnapshots} cpuGroupNames={[{id:'9007199254740993',name:'worker.service'}]}/>);
  const missing = renderToStaticMarkup(<RunMetrics cpuSnapshots={cpuSnapshots}/>);
  // Assert
  assert.match(named, /title="Cgroup 9007199254740993">worker.service/);
  assert.match(named, /worker.service core utilization/);
  assert.match(missing, />Cgroup 9007199254740993<\/span>/);
});

test('elapsed label reports actual run time rather than the timeline window endpoint', () => {
  // Arrange / Act
  const beginning = renderToStaticMarkup(<ActivityTimeline steps={[]} now={0}/>);
  const active = renderToStaticMarkup(<ActivityTimeline steps={[step]} now={2}/>);
  // Assert
  assert.match(beginning,/Elapsed time · 0.0 s/);
  assert.match(active,/Elapsed time · 2.0 s/);
  assert.doesNotMatch(beginning,/Elapsed time · 15.0 s/);
});

test('cancellation during preparation has a terminal result and offers a new run', () => {
  // Arrange
  const snapshot: Snapshot = {phase:'FINISHED',runId:'a'.repeat(32),steps:[],sequence:'0',lostEvents:'0',truncatedSteps:'0',overflowSteps:'0',progressIncomplete:false,cpuAvailable:false,reportAvailable:false,monitorReady:false,systemApps:{ids:[]},outcome:{execution:'Cancelled',boundaryClean:true}};
  // Act
  const html = renderToStaticMarkup(<Result snapshot={snapshot} newRun={() => {}}/>);
  // Assert
  assert.match(html,/Run cancelled/);
  assert.match(html,/>New run<\/button>/);
  assert.doesNotMatch(html,/Stop run|Run complete/);
});
