import { link, mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execution, launch, wire, type NativeExecution } from '../src/native.js';
import { filesystem, fileChangePreview } from '../src/file-changes.js';
import { ConsoleService } from '../src/service.js';
import { Store } from '../src/store.js';
import { History } from '../src/history.js';
import { validateConfig } from '../src/config.js';

/** Real run files, raw names, saved manifests and a reopened Console database. */
export async function runFileFixture() {
  const root = await mkdtemp(join(tmpdir(), 'console-run-files-'));
  const id = 'a'.repeat(32), other = 'b'.repeat(32);
  const runRoot = join(root, 'data/runs', id);
  await mkdir(join(root, 'data'), { mode: 0o700 });
  for (const path of ['filesystem', 'overlays', 'kept/0', 'output']) await mkdir(join(runRoot, path), { recursive: true });
  await mkdir(join(root, 'outside'));
  await writeFile(join(root, 'outside/secret'), 'private');
  await link(join(root, 'outside/secret'), join(runRoot, 'output/hardlink'));
  await mkdir(join(root, 'data/runs', other));
  await writeFile(join(root, 'data/runs', other, 'secret'), 'other run');
  const html = Buffer.from('<script>window.keptHtmlExecuted = true</script><h1>Kept HTML</h1>');
  const rawName = Buffer.from([0x66, 0x69, 0x6c, 0x65, 0x2d, 0xff]);
  await writeFile(join(runRoot, 'kept/0/page.html'), html);
  await writeFile(Buffer.concat([Buffer.from(`${runRoot}/kept/0/`), rawName]), 'raw bytes');
  await writeFile(join(runRoot, 'kept/0/tail.bin'), 'tail');
  await writeFile(join(runRoot, 'output/result.bin'), 'output');
  await writeFile(join(runRoot, 'legacy.mcap'), 'legacy recording');
  await writeFile(join(runRoot, 'console-report.json'), 'private');
  await writeFile(join(runRoot, 'output/private-record'), 'private');
  await writeFile(join(runRoot, 'overlays/results.json'), '{"disposition":"keep"}');
  await symlink(join(root, 'outside'), join(runRoot, 'kept/0/escape'));
  await symlink('page.html', join(runRoot, 'kept/0/link'));
  const row = (source: string, path: string | Buffer, size: number, kind = 1) => filesystem.FileChange.create({
    category: 1, kind, source, path: Buffer.from(path), after: kind === 3 ? undefined : { mode: 0o100600, size },
  });
  const rows = [
    row('/host', 'page.html', html.length), row('/host', rawName, 9), row(join(runRoot, 'output'), 'result.bin', 6),
    row('/host', 'gone', 0, 3), row('/host', 'escape/secret', 7), row('/host', 'link', html.length),
    row(runRoot, 'console-report.json', 7), row(join(root, 'data/runs', other), 'secret', 9),
    row('/host', '../outside', 7), row(join(runRoot, 'output'), 'private-record', 7), row(join(runRoot, 'output'), 'hardlink', 7),
    ...Array.from({ length: 80 }, (_, index) => row('/host', `${index}-${'x'.repeat(300)}`, 0)),
    row('/host', 'tail.bin', 4),
  ];
  const summary = filesystem.CategorySummary.create({ category: 1, counts: { added: rows.length - 1, deleted: 1 }, coverageComplete: true, manifest: 'filesystem/host.pb' });
  const reference = filesystem.FileChangeReference.create({ checkpoint: 'filesystem/report.pb', categories: [summary] });
  await writeFile(join(runRoot, summary.manifest), Buffer.concat(rows.map(value => filesystem.FileChange.encodeDelimited(value).finish())));
  await writeFile(join(runRoot, reference.checkpoint), filesystem.FileChangeReport.encode({
    state: filesystem.ReportState.REPORT_STATE_COMPLETE, categories: [{ summary }], writePolicy: filesystem.WritePolicy.WRITE_POLICY_KEEP,
    overlaySources: [{ index: 0, source: Buffer.from('/host') }], excluded: [Buffer.from('output/private-record')],
  }).finish());
  const config = wire.ConsoleConfig.create({ projectRoot: root, dataRoot: join(root, 'data'), dockerSocket: '/unused',
    writePolicy: filesystem.WritePolicy.WRITE_POLICY_KEEP,
    hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH, listenAddress: '127.0.0.1:8080', publicOrigin: 'http://console.test' });
  await writeFile(join(root, 'compose.yaml'), 'services:\n  worker:\n    image: busybox\n');
  validateConfig(config);
  let store = await Store.open(config.dataRoot);
  const status = wire.Status.create({ runId: id, phase: wire.Phase.FINISHED, writePolicy: filesystem.WritePolicy.WRITE_POLICY_KEEP,
    recordings: [{ id: 'run-file', path: 'legacy.mcap' }],
    outcome: { execution: 'Completed', recordingFinalized: true, boundaryClean: true }, fileChanges: await fileChangePreview(runRoot, reference) });
  new History(store).record(status, 1, 2);
  store.close(); store = await Store.open(config.dataRoot);
  const engine: NativeExecution = {
    composeProfiles: async () => execution.ComposeProfiles.create(), inspect: async () => execution.Inspection.create({
      plan: { services: { worker: { imageId: 'sha256:fixture' } } }, composeSnapshot: { manifestSha256: 'fixture' },
    }),
    start: async () => {}, stop: async () => {}, poll: () => execution.ExecutionUpdate.create(), close: async () => {},
  };
  const backend = new ConsoleService(config, engine, store);
  return { root, runRoot, id, other, html, rawName, rows, reference, status, backend,
    close: async () => { await backend.close(); await rm(root, { recursive: true, force: true }); } };
}
