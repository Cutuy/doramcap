import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import Long from 'long';
import { filesystem } from '../src/file-changes.js';

/** Write actual delimited manifest bytes, including paths that cannot be UTF-8; `layerCount` adds container-layer records. */
export async function fileChangeFixture(root: string, count = 3, notes = ['source excluded: live shared memory'], layerCount = 0) {
  await mkdir(join(root, 'filesystem'), { recursive: true, mode: 0o700 });
  const records = Array.from({ length: count }, (_, index) => filesystem.FileChange.create({
    category: 1, kind: 1, source: '/source', consumers: ['project/writer:/out', 'project/reader:/in'],
    path: Buffer.concat([Buffer.from(`file-${index}-`), Buffer.from([0xff])]),
    after: { size: Long.fromString('9223372036854775807', true), mode: 0o100600,
      mtimeSeconds: Long.fromString('-9223372036854775808'), symlink: Buffer.from([0xfe]) },
  }));
  await writeFile(join(root, 'filesystem/host.pb'), Buffer.concat(records.map(row => filesystem.FileChange.encodeDelimited(row).finish())));
  const summary = filesystem.CategorySummary.create({
    category: 1, counts: { added: count }, coverageComplete: !notes.length,
    manifest: 'filesystem/host.pb', coverageNoteCount: notes.length,
  });
  const layers = Array.from({ length: layerCount }, (_, index) => filesystem.FileChange.create({
    category: 2, kind: 2, source: 'container', path: Buffer.from(`/layer-${index}`),
  }));
  const layerSummary = filesystem.CategorySummary.create({
    category: 2, counts: { modified: layerCount }, coverageComplete: true, manifest: 'filesystem/layers.pb',
  });
  if (layerCount) await writeFile(join(root, 'filesystem/layers.pb'), Buffer.concat(layers.map(row => filesystem.FileChange.encodeDelimited(row).finish())));
  const summaries = layerCount ? [summary, layerSummary] : [summary];
  const reference = filesystem.FileChangeReference.create({ checkpoint: 'filesystem/report.pb', categories: summaries });
  await writeFile(join(root, reference.checkpoint), filesystem.FileChangeReport.encode({ state: filesystem.ReportState.REPORT_STATE_COMPLETE,
    categories: summaries.map(item => ({ summary: item, coverageNotes: item === summary ? notes : [] })) }).finish());
  return { reference, records, layers };
}
