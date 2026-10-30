import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { App } from './App';
import { Result } from './Result';
import { RunSteps } from './RunSteps';
import type { Snapshot } from './api';

test('project screen has labeled controls and phase navigation', () => {
  // Arrange / Act
  const html = renderToStaticMarkup(<App />);
  // Assert
  assert.match(html, /aria-label="Run phases"/);
  assert.match(html, /for="directory"/);
  assert.match(html, /Continue/);
});

test('blocked cleanup preserves saved recording and disables new run', () => {
  // Arrange
  const snapshot = { phase: 'BLOCKED', runId: 'a'.repeat(32), outcome: { execution: 'Failed', recordingFinalized: true, boundaryClean: false }, cpuAvailable: true, reportAvailable: true } as Snapshot;
  // Act
  const html = renderToStaticMarkup(<Result snapshot={snapshot} newRun={() => {}} />);
  // Assert
  assert.match(html, /Cleanup unverified/);
  assert.match(html, /Saved/);
  assert.match(html, /Download CPU recording/);
  assert.match(html, /disabled=""[^>]*>New run/);
});

test('repeated workflow invocations remain separate and retain full identity', () => {
  // Arrange
  const rows = ['9007199254740993', '9007199254740994'].map(invocation => ({ session: 's', executor: 'local', invocation, step: 'module::ObserveProjects', state: 'SUCCEEDED', durationNs: '100000000' }));
  // Act
  const html = renderToStaticMarkup(<RunSteps steps={rows} />);
  // Assert
  assert.equal((html.match(/Observe application/g) ?? []).length, 2);
  assert.match(html, /9007199254740993/);
  assert.match(html, /module::ObserveProjects/);
});
