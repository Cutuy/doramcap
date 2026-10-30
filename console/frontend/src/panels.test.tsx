import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { ActivityTimeline, EventDetails } from './ActivityTimeline';
import { RunActivity } from './RunActivity';
import { RunMetrics } from './RunMetrics';
import { Result } from './Result';
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
  assert.match(html, /private host evidence/);
  assert.match(html, /payloads are excluded/);
});
test('timeline events are keyboard buttons and completed view supports elapsed-time browsing', () => {
  // Arrange / Act
  const html = renderToStaticMarkup(<ActivityTimeline steps={[step]} now={2} live={false}/>);
  // Assert
  assert.match(html, /role="button" tabindex="0"/);
  assert.match(html, /Browse elapsed time/);
  assert.match(html, /Retained workflow events from this run/);
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
  assert.match(absent, /Unavailable/);
  assert.match(absent, /Connection lost/);
  assert.match(absent, /incomplete/);
  assert.match(zero, /0 B/);
  assert.match(zero, /0.0%/);
  assert.match(zero, /8 cores/);
  assert.match(zero, /100% equals one/);
});

test('completed runs without retained activity do not imply that events will arrive', () => {
  // Arrange
  const snapshot: Snapshot = { phase: 'FINISHED', runId: 'old-run', steps: [], sequence: '0', truncatedSteps: '0', lostEvents: '0', overflowSteps: '0', progressIncomplete: false, outcome: null, cpuAvailable: false, reportAvailable: false, monitorReady: false };
  // Act
  const html = renderToStaticMarkup(<RunActivity snapshot={snapshot} historical stopping={false} disconnected={false}/>);
  // Assert
  assert.match(html, /No workflow steps were retained/);
  assert.match(html, /No retained workflow events available/);
  assert.doesNotMatch(html, /Waiting for workflow/);
});
