import { wire, workflow } from './api';
import { Phase, StepState } from './api';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { ActivityTimeline, EventDetails } from './ActivityTimeline';
import { RunActivity } from './RunActivity';
import { RunSteps } from './RunSteps';
import { RunMetrics } from './RunMetrics';
import { Result } from './Result';
import { SystemApps } from './SystemApps';
import { selectedProfiles } from './model';
import { cpuRecordingEnabled } from './api';
import { doramcap } from './generated/console';
import { PageFrame } from './ui/PageFrame';
import { Kind, LiveModel } from './streams';
import type { Snapshot, Step } from './api';

const step: Step = workflow.Event.fromObject({ session: 'session', executor: 'local', invocation: '9007199254740993', step: 'module::LaunchProjects', state: StepState.FAILED, durationNs: '100000000', elapsedNs: '1200000000', parent: '9007199254740994', dependencies: ['9007199254740995'], sequence: '9007199254740996', error: 'compose up exited 1: no such image' });
test('event inspector preserves precise identity, timing, relationships and failure information', () => {
  // Arrange / Act
  const html = renderToStaticMarkup(<EventDetails step={step} now={3}/>);
  // Assert
  for (const id of ['9007199254740993', '9007199254740994', '9007199254740995', '9007199254740996']) assert.match(html, new RegExp(id));
  assert.match(html, /0.100 s/);
  assert.match(html, /1.100 s/);
  assert.match(html, /no such image/);
  assert.doesNotMatch(html, /payloads are excluded|private host evidence/);
});
test('the step table shows why a step failed', () => {
  // Arrange / Act
  const html = renderToStaticMarkup(<RunSteps steps={[workflow.Event.create({ ...step, parent: null })]}/>);
  // Assert
  assert.match(html, /Failed/);
  assert.match(html, /compose up exited 1: no such image/);
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
  const snapshot = wire.Status.fromObject({ phase: Phase.BLOCKED, runId: 'a'.repeat(32), outcome: null, cpuAvailable: false, reportAvailable: false });
  // Act
  const html = renderToStaticMarkup(<Result snapshot={snapshot} newRun={() => {}} viewRun={() => {}}/>);
  const rail = renderToStaticMarkup(<PageFrame phase="Result" onNavigate={() => {}}>Result</PageFrame>);
  // Assert
  assert.match(html, /View run activity/);
  assert.match(html, />New run<\/button>/);
  assert.match(rail, /<button[^>]*><small>03<\/small>Run<\/button>/);
});
test('historical run freezes elapsed time and unavailable metrics remain distinct from zero', () => {
  // Arrange
  const snapshot = wire.Status.fromObject({ phase: Phase.FINISHED, runId: 'run', steps: [step], sequence: '1', truncatedSteps: '0', outcome: { execution: 'Completed' } });
  // Act
  const historical = renderToStaticMarkup(<RunActivity snapshot={snapshot} stopping={false} disconnected={false} historical/>);
  const model = new LiveModel();
  model.configure('run', [wire.LiveStream.create({ id: 'cpu_0', appId: 'cpu', kind: Kind.CPU_ACCOUNTING })]);
  const absent = renderToStaticMarkup(<RunMetrics live={model.view()} disconnected incomplete/>);
  const zero = renderToStaticMarkup(<RunMetrics samples={[wire.RunMetric.fromObject({elapsedNs:'1000000000',recordingBytes:'0',writeBytesPerSecond:0})]} live={model.view()} historical/>);
  // Assert
  assert.match(historical, /Completed run/);
  assert.doesNotMatch(historical, /Retained view|512 invocations|1200 metric samples/);
  assert.match(absent, /Unavailable/);
  assert.match(absent, /Connection lost/);
  assert.match(absent, /incomplete/);
  assert.match(zero, /0 B/);
  assert.match(zero, /0 B\/s/);
  assert.doesNotMatch(zero, /Connection lost/);
  assert.match(zero, /Top CPU consumers/);
  assert.match(zero, /Capture health/);
  assert.doesNotMatch(zero, /Inclusive ancestors|recorder boundary/);
});

test('a stopping run names the services it is waiting on until they exit', () => {
  // Arrange
  const stopping: Snapshot = wire.Status.fromObject({ phase: Phase.STOPPING, runId: 'run', steps: [], sequence: '0', truncatedSteps: '0', stoppingServices: [{ service: 'perception', forceStopInSeconds: 12 }, { service: 'control' }] });
  const exited: Snapshot = wire.Status.fromObject({ phase: Phase.STOPPING, runId: 'run', steps: [], sequence: '0', truncatedSteps: '0' });
  // Act
  const waiting = renderToStaticMarkup(<RunActivity snapshot={stopping} stopping disconnected={false}/>);
  const none = renderToStaticMarkup(<RunActivity snapshot={exited} stopping disconnected={false}/>);
  const past = renderToStaticMarkup(<RunActivity snapshot={stopping} stopping={false} disconnected={false} historical/>);
  // Assert
  assert.match(waiting, /Waiting for services to stop/);
  assert.match(waiting, /perception<\/span> <span[^>]*>force stop in 12 s/);
  assert.match(waiting, /control<\/span> <span[^>]*>force stopping/);
  assert.doesNotMatch(none, /Waiting for services/);
  assert.doesNotMatch(past, /Waiting for services/);
});

test('completed runs without retained activity do not imply that events will arrive', () => {
  // Arrange
  const snapshot: Snapshot = wire.Status.fromObject({ phase: Phase.FINISHED, runId: 'old-run', steps: [], sequence: '0', truncatedSteps: '0', lostEvents: '0', overflowSteps: '0', progressIncomplete: false, outcome: null, cpuAvailable: false, reportAvailable: false, monitorReady: false });
  // Act
  const html = renderToStaticMarkup(<RunActivity snapshot={snapshot} historical stopping={false} disconnected={false}/>);
  // Assert
  assert.match(html, /No workflow steps available/);
  assert.match(html, /No events available/);
  assert.doesNotMatch(html, /Waiting for workflow/);
});


test('system app choices reflect selection and disabled recording stays distinct from failure', () => {
  // Arrange
  const apps = [wire.SystemApp.fromObject({ id: 'cpu-monitor', label: 'CPU recording', selected: true })];
  const snapshot: Snapshot = wire.Status.fromObject({ phase: Phase.FINISHED, runId: 'a'.repeat(32), steps: [], sequence: '0', lostEvents: '0', truncatedSteps: '0', overflowSteps: '0', progressIncomplete: false, cpuAvailable: false, reportAvailable: true, monitorReady: false, systemApps: { ids: [] }, outcome: { execution: 'Completed', recordingFinalized: null, boundaryClean: true } });
  // Act
  const enabled = renderToStaticMarkup(<SystemApps apps={apps} busy={false} toggle={() => {}} chooseProfile={() => {}}/>);
  const checking = renderToStaticMarkup(<SystemApps apps={[wire.SystemApp.fromObject({...apps[0],selected:false})]} busy toggle={() => {}} chooseProfile={() => {}}/>);
  const result = renderToStaticMarkup(<Result snapshot={snapshot} newRun={() => {}}/>);
  // Assert
  assert.match(enabled, /type="checkbox" checked=""/);
  assert.match(enabled, /CPU recording/);
  assert.match(checking, /fieldset disabled=""/);
  assert.doesNotMatch(checking, /checked=""/);
  assert.match(result, /Disabled/);
  assert.match(result, /Recording disabled/);
  assert.doesNotMatch(result, /Download CPU recording/);
  assert.equal(cpuRecordingEnabled(snapshot), false);
  assert.equal(cpuRecordingEnabled(wire.Status.fromObject({ recordings: [{id:'metrics',path:'metrics.mcap',cpu:true}] })), true);
  assert.equal(cpuRecordingEnabled(wire.Status.fromObject({})), false);
});

test('CPU consumers show reported cgroup names with an ID tooltip and numeric fallback', () => {
  // Arrange
  const telemetry = doramcap.telemetry.v1;
  const model = new LiveModel();
  model.configure('run', [wire.LiveStream.create({ id: 'cpu_0', appId: 'cpu', kind: Kind.CPU_ACCOUNTING })]);
  const packet = (sequence: number, dictionary: object[] = []) => doramcap.console.live.v1.Frame.create({ stream: 'cpu_0', sequence, payload: telemetry.Packet.encode(telemetry.Packet.fromObject({ snapshot: { version: 2, sessionId: 's', dictionary, snapshot: { sequence, monotonicEndNs: sequence * 100000000, records: [{ cgroupId: '9007199254740993', parentId: 0, cpu: 0, flags: 1, runtimeNs: sequence * 10000000 }] } } })).finish() });
  model.receive(packet(1));
  model.receive(packet(2));
  const missing = renderToStaticMarkup(<RunMetrics live={model.view()}/>);
  // Act
  model.receive(packet(3, [{ id: '9007199254740993', path: '/system.slice/worker.service' }]));
  const named = renderToStaticMarkup(<RunMetrics live={model.view()}/>);
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
  const snapshot: Snapshot = wire.Status.fromObject({phase:Phase.FINISHED,runId:'a'.repeat(32),steps:[],sequence:'0',lostEvents:'0',truncatedSteps:'0',overflowSteps:'0',progressIncomplete:false,cpuAvailable:false,reportAvailable:false,monitorReady:false,systemApps:{ids:[]},outcome:{execution:'Cancelled',boundaryClean:true}});
  // Act
  const html = renderToStaticMarkup(<Result snapshot={snapshot} newRun={() => {}}/>);
  // Assert
  assert.match(html,/Run cancelled/);
  assert.match(html,/>New run<\/button>/);
  assert.doesNotMatch(html,/Stop run|Run complete/);
});


test('required infrastructure is selected and disabled independently of optional app choices', () => {
  // Arrange
  const apps = [wire.SystemApp.fromObject({id:'dora-daemon',label:'Dora IPC daemon',selected:true,required:true}),wire.SystemApp.fromObject({id:'recorder',label:'MCAP recorder',selected:false})];
  // Act
  const html = renderToStaticMarkup(<SystemApps apps={apps} busy={false} toggle={() => {}} chooseProfile={() => {}}/>);
  // Assert
  assert.match(html, /type="checkbox" disabled="" checked=""/);
  assert.match(html, /Required/);
  assert.equal((html.match(/disabled=""/g) ?? []).length, 1);
});

test('runs without live streams show CPU and recording panels as disabled', () => {
  // Arrange
  const samples = [wire.RunMetric.fromObject({ elapsedNs: '1000000000' })];
  // Act
  const html = renderToStaticMarkup(<RunMetrics samples={samples} recordingEnabled={false} live={new LiveModel().view()}/>);
  // Assert
  assert.equal((html.match(/>Disabled</g) ?? []).length, 4);
  assert.doesNotMatch(html, /Faults/);
  assert.doesNotMatch(html, /Unavailable/);
});

test('system apps offer a profile picker only when an app has several profiles', () => {
  // Arrange
  const profiles = [{ id: 'record', label: 'Record' }, { id: 'live', label: 'Live only' }];
  const apps = [wire.SystemApp.fromObject({ id: 'monitor', label: 'Monitor', selected: true, profiles, profile: 'live' })];
  const off = [wire.SystemApp.fromObject({ ...apps[0], selected: false })];
  const single = [wire.SystemApp.fromObject({ ...apps[0], profiles: profiles.slice(0, 1), profile: 'record' })];
  // Act
  const html = renderToStaticMarkup(<SystemApps apps={apps} busy={false} toggle={() => {}} chooseProfile={() => {}}/>);
  const disabled = renderToStaticMarkup(<SystemApps apps={off} busy={false} toggle={() => {}} chooseProfile={() => {}}/>);
  const none = renderToStaticMarkup(<SystemApps apps={single} busy={false} toggle={() => {}} chooseProfile={() => {}}/>);
  // Assert
  assert.match(html, /aria-label="Monitor profile"/);
  assert.match(html, /<option value="live" selected="">Live only<\/option>/);
  assert.match(disabled, /<select[^>]*disabled=""/);
  assert.doesNotMatch(none, /<select/);
  assert.deepEqual(selectedProfiles([...apps, wire.SystemApp.fromObject({ id: 'other', selected: true })]), { monitor: 'live' });
  assert.deepEqual(selectedProfiles(off), {});
});

test('a saved recording reports how killed services exited', () => {
  // Arrange
  const status = (serviceExits: object[]) => wire.Status.fromObject({ phase: Phase.FINISHED, runId: 'a'.repeat(32), recordings: [{ id: 'cpu', path: 'cpu.mcap', cpu: true }], reportAvailable: true, outcome: { execution: 'Completed', boundaryClean: true, recordingFinalized: true, serviceExits } }) as Snapshot;
  // Act
  const killed = renderToStaticMarkup(<Result snapshot={status([{ service: 'perception', exitCode: 137, killed: true }, { service: 'control', exitCode: 137, oomKilled: true, killed: true }, { service: 'planner', exitCode: 143 }])} newRun={() => {}}/>);
  const clean = renderToStaticMarkup(<Result snapshot={status([{ service: 'planner', exitCode: 143 }])} newRun={() => {}}/>);
  // Assert
  assert.match(killed, /Saved/);
  assert.match(killed, /Download cpu recording/);
  assert.match(killed, /Killed: perception \(exit 137\), control \(exit 137, out of memory\)\. Final messages may be missing/);
  assert.doesNotMatch(killed, /planner/);
  assert.doesNotMatch(clean, /Killed:/);
});
