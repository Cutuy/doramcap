import { strict as assert } from 'node:assert';
import { execFileSync } from 'node:child_process';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { sourceState } from '../src/source.js';
import { doramcap } from '../src/generated/console.js';

const { SourceState } = doramcap.console.execution.v1;

const temporary = () => mkdtemp(join(tmpdir(), 'console-source-'));

test('a project reports its commit and local changes, and a non-Git one reports neither', async () => {
  // Arrange
  const project = await temporary(), plain = await temporary();
  const git = (...args: string[]) => execFileSync('git', ['-C', project, '-c', 'user.name=t', '-c', 'user.email=t@example.com', ...args]).toString().trim();
  git('init', '-q');
  await writeFile(join(project, 'compose.yaml'), 'services: {}');
  git('add', '.');
  git('commit', '-qm', 'first');
  // Act
  const clean = await sourceState(project);
  await writeFile(join(project, 'compose.yaml'), 'services: { a: {} }');
  const dirty = await sourceState(project);
  const none = await sourceState(plain);
  await writeFile(join(project, '.git/index'), 'corrupt');
  const statusFailed = await sourceState(project);
  // Assert
  assert.deepEqual(clean, { sourceCommit: git('rev-parse', 'HEAD'), sourceState: SourceState.SOURCE_STATE_CLEAN });
  assert.equal(dirty.sourceState, SourceState.SOURCE_STATE_DIRTY);
  assert.deepEqual(none, { sourceState: SourceState.SOURCE_STATE_UNKNOWN });
  assert.deepEqual(statusFailed, { sourceCommit: clean.sourceCommit, sourceState: SourceState.SOURCE_STATE_UNKNOWN });
});

test('unknown, clean and dirty source states survive capture serialization', () => {
  // Arrange
  const { ExperimentSetup } = doramcap.console.execution.v1;
  for (const sourceState of [SourceState.SOURCE_STATE_UNKNOWN, SourceState.SOURCE_STATE_CLEAN, SourceState.SOURCE_STATE_DIRTY]) {
    // Act
    const saved = ExperimentSetup.decode(ExperimentSetup.encode({ sourceState }).finish());
    // Assert
    assert.equal(saved.sourceState, sourceState);
  }
});
