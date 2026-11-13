import React from 'react';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { renderToStaticMarkup } from 'react-dom/server';
import { doramcap } from './generated/console';
import { WritableFolders } from './WritableFolders';
import { Result } from './Result';
import { wire } from './api';

const { SourceKind: Kind, WritePolicy: Policy } = doramcap.filesystem.v1;
const folders = [
  { source: '/data/shared', apps: ['camera', 'recorder'], kind: Kind.SOURCE_KIND_OVERLAY },
  { source: '/run/robot', apps: ['camera'], kind: Kind.SOURCE_KIND_PASSTHROUGH },
  { source: '/proc', apps: ['monitor'], kind: Kind.SOURCE_KIND_UNTRACKED, reason: 'not tracked: pseudo-filesystem or device mount' },
];

for (const [policy, label] of [[Policy.WRITE_POLICY_WRITE_BACK, 'Write back'], [Policy.WRITE_POLICY_KEEP, 'Keep with the run'], [Policy.WRITE_POLICY_DISCARD, 'Discard']] as const) test(`writable folders separate live handling from ${label}`, () => {
  // Arrange / Act
  const html = renderToStaticMarkup(<WritableFolders folders={folders} policy={policy}/>);
  // Assert
  for (const column of ['Host folder', 'App', 'During the run', 'After the run']) assert.ok(html.includes(column));
  assert.match(html, /camera, recorder/);
  assert.match(html, new RegExp(`Overlay<\/td><td data-label="After the run">${label}`));
  assert.match(html, /Passed through<\/td><td data-label="After the run">Already on the host; no change list/);
  assert.match(html, /Not tracked: pseudo-filesystem or device mount/);
  assert.match(html, /Write policy:/);
  assert.match(html, /passthrough_folders: &quot;\/data\/shared&quot;/);
  assert.match(html, /Copy passthrough setting for \/run\/robot/);
  assert.doesNotMatch(html, /<(input|select|textarea)|role="switch"/);
  assert.ok(html.indexOf('Write policy:') > html.indexOf('</table>'));
});

test('empty and unknown handling stay explicit; configuration paths retain their escapes', () => {
  // Arrange / Act
  const empty = renderToStaticMarkup(<WritableFolders folders={[]}/>);
  const unknown = renderToStaticMarkup(<WritableFolders folders={[{ source: '/a/"quoted"', kind: 99 as typeof Kind.SOURCE_KIND_UNSPECIFIED }]}/>);
  // Assert
  assert.match(empty, /No writable host folders/);
  assert.match(empty, /passthrough_folders: &quot;&lt;folder&gt;&quot;/);
  assert.match(unknown, /Not tracked: handling unavailable/);
  assert.ok(unknown.includes('\\&quot;quoted\\&quot;'));
});

test('Result renders the saved folder handling and policy', () => {
  // Arrange / Act
  const snapshot = wire.Status.create({ writableFolders: folders.map(folder => doramcap.filesystem.v1.WritableFolder.create(folder)), writePolicy: Policy.WRITE_POLICY_KEEP });
  const html = renderToStaticMarkup(<Result snapshot={snapshot} newRun={() => {}}/>);
  // Assert
  assert.match(html, /Writable folders/); assert.match(html, /\/data\/shared/); assert.match(html, /Keep with the run/);
});
