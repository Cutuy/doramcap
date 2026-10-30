import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import type { Step } from './api';
import { ActivityTimeline } from './ActivityTimeline';
import { flowPath, timelineModel } from './timeline';

const step = (invocation: string, name: string, extra: Partial<Step> = {}): Step => ({ session: 'run', executor: 'local', invocation, step: name, state: 'SUCCEEDED', elapsedNs: '2000000000', durationNs: '1000000000', ...extra });

test('each full step name owns one row across repeated invocations and window changes', () => {
  // Arrange
  const steps = [step('1', 'compose::ObserveProjects'), step('2', 'compose::InspectCompose'), step('3', 'compose::InspectCompose'), step('4', 'other::InspectCompose')];
  // Act
  const model = timelineModel(steps);
  const html = renderToStaticMarkup(<ActivityTimeline steps={steps} now={30} />);
  // Assert
  assert.deepEqual(model.rows, ['compose::ObserveProjects', 'compose::InspectCompose', 'other::InspectCompose']);
  assert.equal(model.rowByName.get(steps[1].step), model.rowByName.get(steps[2].step));
  assert.equal((html.match(/class="timeline-step-row"/g) ?? []).length, 3);
  assert.equal((html.match(/class="timeline-event"/g) ?? []).length, 0);
});

test('flows use exact scoped IDs and reported parent/dependency timing only', () => {
  // Arrange
  const source = step('9007199254740993', 'compose::Prepare');
  const target = step('9007199254740994', 'compose::InspectCompose', { parent: source.invocation, dependencies: [source.invocation, source.invocation], elapsedNs: '4000000000' });
  const unrelated = step('9007199254740995', 'compose::Prepare', { session: 'other', parent: source.invocation });
  const otherExecutor = step('5', 'compose::InspectCompose', { executor: 'other', dependencies: [source.invocation] });
  // Act
  const { flows } = timelineModel([source, target, unrelated, otherExecutor]);
  // Assert
  assert.deepEqual(flows.map(flow => [flow.kind, flow.source.invocation, flow.target.invocation, flow.from, flow.to]), [
    ['parent', source.invocation, target.invocation, 1, 3],
    ['dependency', source.invocation, target.invocation, 2, 3],
  ]);
  // Act / Assert: eviction never creates a substitute edge.
  assert.deepEqual(timelineModel([target, unrelated, otherExecutor]).flows, []);
});

test('same-row arrows remain curved and explicit links crossing the window are rendered', () => {
  // Arrange
  const source = step('1', 'compose::InspectCompose');
  const target = step('2', source.step, { dependencies: ['1'], elapsedNs: '17000000000' });
  // Act
  const path = flowPath(10, 46, 30, 46);
  const html = renderToStaticMarkup(<ActivityTimeline steps={[source, target]} now={20} live={false} />);
  // Assert
  assert.equal(path, 'M10,46 C10,21 30,21 30,46');
  assert.equal((html.match(/class="timeline-flow dependency"/g) ?? []).length, 1);
  assert.match(html, /marker-end="url\(#activity-arrow-/);
  assert.match(html, /Dependency: InspectCompose \(1\) → InspectCompose \(2\)/);
  assert.equal((html.match(/class="timeline-event"/g) ?? []).length, 1);
});


test('timeline rows follow recorded start time even when startup events arrive after cleanup', () => {
  // Arrange: terminal updates and restored rows can arrive in a different order.
  const steps = [step('3', 'compose::WriteRunReport', {elapsedNs:'10000000000',durationNs:'1000000000'}), step('2', 'compose::QuiesceCompose', {elapsedNs:'8000000000',durationNs:'1000000000'}), step('1', 'compose::StartCompose'), step('4', 'compose::StartCompose', {elapsedNs:'3000000000',durationNs:'1000000000'})];
  // Act
  const { rows } = timelineModel(steps);
  // Assert
  assert.deepEqual(rows, ['compose::StartCompose', 'compose::QuiesceCompose', 'compose::WriteRunReport']);
});


test('stable step IDs keep one timeline row across label changes', () => {
  // Arrange
  const steps = [step('1', 'custom.stage', { displayName: 'Original label' }), step('2', 'custom.stage', { displayName: 'Custom stage' })];
  // Act
  const model = timelineModel(steps);
  const html = renderToStaticMarkup(<ActivityTimeline steps={steps} now={2} live={false}/>);
  // Assert
  assert.deepEqual(model.rows, ['custom.stage']);
  assert.equal((html.match(/class="timeline-step-row"/g) ?? []).length, 1);
  assert.match(html, /Custom stage/);
});
