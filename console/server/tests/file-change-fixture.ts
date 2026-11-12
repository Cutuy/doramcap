import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import Long from 'long';
import { filesystem } from '../src/file-changes.js';

/** Write actual delimited manifest bytes, including paths that cannot be UTF-8. */
export async function fileChangeFixture(root: string, count = 3, notes = ['source excluded: live shared memory']) {
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
  const reference = filesystem.FileChangeReference.create({ checkpoint: 'filesystem/report.pb', categories: [summary] });
  await writeFile(join(root, reference.checkpoint), filesystem.FileChangeReport.encode({ state: filesystem.ReportState.REPORT_COMPLETE, categories: [{ summary, coverageNotes: notes }] }).finish());
  return { reference, records };
}
