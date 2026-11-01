import { wire, workflow } from './api';
import { Phase, StepState } from './api';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { App } from './App';
import { Result } from './Result';
import { RunSteps } from './RunSteps';
import { ActivityTimeline } from './ActivityTimeline';
import type { Snapshot, Step } from './api';

test('project screen has labeled controls and phase navigation', () => {
  // Arrange / Act
  const html = renderToStaticMarkup(<App />);
  // Assert
  assert.match(html, /aria-label="Run phases"/);
  assert.match(html, /for="directory"/);
  assert.match(html, /Continue/);
});

test('activity timeline retains repeated health-check invocations', () => {
  // Arrange
  const rows: Step[] = ['9007199254740993', '9007199254740994'].map((invocation, index) => (workflow.Event.fromObject({ session: 's', executor: 'local', invocation, step: 'module::ObserveProjects', state: StepState.SUCCEEDED, elapsedNs: String((index + 1) * 1_000_000_000), durationNs: '100000000' })));
  // Act
  const html = renderToStaticMarkup(<ActivityTimeline steps={rows} now={2} />);
  // Assert
  assert.equal((html.match(/class="timeline-step-row"/g) ?? []).length, 1);
  assert.match(html, /data-step="module::ObserveProjects"/);
  assert.match(html, /invocation 9007199254740993/);
  assert.match(html, /invocation 9007199254740994/);
});

test('blocked cleanup preserves declared recordings and allows another reset attempt', () => {
  // Arrange
  const snapshot = wire.Status.fromObject({ phase: Phase.BLOCKED, runId: 'a'.repeat(32), outcome: { execution: 'Failed', recordingFinalized: true, boundaryClean: false }, cpuAvailable: true, reportAvailable: true, recordings: [{id:'cpu',path:'cpu.mcap',cpu:true},{id:'dora',path:'dora.mcap'}] });
  // Act
  const html = renderToStaticMarkup(<Result snapshot={snapshot} newRun={() => {}} />);
  // Assert
  assert.match(html, /Cleanup unverified/);
  assert.match(html, /Saved/);
  assert.match(html, /Download cpu recording/);
  assert.match(html, />New run<\/button>/);
  assert.match(html, /Download dora recording/);
  assert.match(html, /Reset required/);
});

test('workflow list summarizes latest top-level invocations and excludes children', () => {
  // Arrange
  const rows: Step[] = ['9007199254740993', '9007199254740994'].map(invocation => (workflow.Event.fromObject({ session: 's', executor: 'local', invocation, step: 'compose.launch', displayName: 'Start project', state: StepState.SUCCEEDED, durationNs: '100000000', elapsedNs: '200000000' })));
  rows.push(workflow.Event.fromObject({ session: 's', executor: 'local', invocation: '3', step: 'module::ObserveProjects', state: StepState.SUCCEEDED, durationNs: '100000000', elapsedNs: '300000000' }));
  rows.push(workflow.Event.fromObject({ ...rows[0], invocation: '4', parent: '3', step: 'module::InspectCompose', displayName: null }));
  // Act
  const html = renderToStaticMarkup(<RunSteps steps={rows} />);
  // Assert
  assert.equal((html.match(/Start project/g) ?? []).length, 1);
  assert.match(html, /9007199254740994/);
  assert.match(html, /ObserveProjects/);
  assert.doesNotMatch(html, /InspectCompose/);
});
