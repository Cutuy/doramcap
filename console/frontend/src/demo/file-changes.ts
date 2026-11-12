import { wire, type Snapshot } from '../api';
import { displayPath } from '../generated/transport';
import { doramcap } from '../generated/console';
import { digest } from './telemetry';

const filesystem = doramcap.filesystem.v1;
const { StorageCategory: Category, ChangeKind: Change, ReportState } = filesystem;
const HOST = Category.STORAGE_CATEGORY_HOST_BIND, LAYER = Category.STORAGE_CATEGORY_CONTAINER_LAYER;
const ADDED = Change.CHANGE_KIND_ADDED, MODIFIED = Change.CHANGE_KIND_MODIFIED, DELETED = Change.CHANGE_KIND_DELETED;
const MILLISECONDS_PER_SECOND = 1000, NANOSECONDS_PER_MILLISECOND = 1_000_000;
const PREVIOUS_CAPTURE_AGE_MS = 60_000;
const MIB = 1024 ** 2;
const JSON_OPTIONS = { longs: String, bytes: String, enums: String, defaults: true } as const;
const CATEGORIES = [
  { category: HOST, manifest: 'filesystem/host-binds.pb', notes: [] },
  { category: LAYER, manifest: 'filesystem/container-layers.pb', notes: [] },
  { category: Category.STORAGE_CATEGORY_NAMED_VOLUME, manifest: 'filesystem/named-volumes.pb', notes: [
    'perception-model-cache: named volume is not tracked; writes to /models are not listed.',
  ] },
];

/** A compact, complete preview of synthetic changes, with stat times tied to this run. */
export function demoFileChanges(status: Snapshot, startedAtMs: number, finishedAtMs: number) {
  const project = status.projectDirectory.split('/').at(-1)!;
  const metadata = (size: number, time: number, fields: doramcap.filesystem.v1.IFileMetadata = {}) => {
    const seconds = Math.floor(time / MILLISECONDS_PER_SECOND);
    const nanoseconds = (time - seconds * MILLISECONDS_PER_SECOND) * NANOSECONDS_PER_MILLISECOND;
    return filesystem.FileMetadata.fromObject({
      mode: 0o100644, size, uid: 1000, gid: 1000,
      mtimeSeconds: seconds, mtimeNanoseconds: nanoseconds, ctimeSeconds: seconds, ctimeNanoseconds: nanoseconds, ...fields,
    });
  };
  const previous = (size: number, fields: doramcap.filesystem.v1.IFileMetadata = {}) => metadata(size, startedAtMs - PREVIOUS_CAPTURE_AGE_MS, fields);
  const current = (size: number, fields: doramcap.filesystem.v1.IFileMetadata = {}) => metadata(size, finishedAtMs, fields);
  const changed = current(0);
  const changedTime = { ctimeSeconds: changed.ctimeSeconds, ctimeNanoseconds: changed.ctimeNanoseconds };
  const pathBytes = (path: string) => new TextEncoder().encode(path);
  const symlink = (target: string) => ({ mode: 0o120777, symlink: pathBytes(target) });
  const previousTarget = 'frames/frame-000041.pcd', currentTarget = 'frames/frame-000042.pcd';
  const preview: doramcap.filesystem.v1.FileChange[] = [];
  const source = (category: number, identity: string, consumers: string[], changes: doramcap.filesystem.v1.IFileChange[]) => {
    preview.push(...changes.map(change => filesystem.FileChange.create({ ...change, category, source: identity, consumers })));
  };

  source(HOST, `${status.projectDirectory}/config`, [`${project}/sensor-fusion:/config`, `${project}/object-detector:/config`], [
    { path: pathBytes('perception.yaml'), kind: MODIFIED, before: previous(1204), after: current(1388) },
    { path: pathBytes('calibration/camera.yaml'), kind: MODIFIED, before: previous(2048), after: current(2112) },
    { path: pathBytes('runtime.yaml'), kind: MODIFIED, before: previous(256), after: previous(256, { mode: 0o100600, ...changedTime }) },
    { path: pathBytes('calibration/lidar.yaml'), kind: ADDED, after: current(896) },
    { path: pathBytes('calibration/camera.bak'), kind: DELETED, before: previous(2048) },
  ]);
  source(HOST, `${status.projectDirectory}/results`, [`${project}/object-detector:/results`, `${project}/sensor-fusion:/results`], [
    { path: pathBytes('frames'), kind: ADDED, after: current(4096, { mode: 0o040755 }) },
    { path: pathBytes('frames/frame-000042.pcd'), kind: ADDED, after: current(2.5 * MIB) },
    { path: pathBytes('detections.jsonl'), kind: ADDED, after: current(18_432) },
    { path: pathBytes('summary.json'), kind: MODIFIED, before: previous(512), after: current(768) },
    { path: pathBytes('session.lock'), kind: MODIFIED, before: previous(32), after: previous(32, { uid: 0, gid: 0, ...changedTime }) },
    { path: pathBytes('latest'), kind: MODIFIED,
      before: previous(pathBytes(previousTarget).length, symlink(previousTarget)),
      after: current(pathBytes(currentTarget).length, symlink(currentTarget)) },
    { path: pathBytes('stale-detections.jsonl'), kind: DELETED, before: previous(49_152) },
  ]);

  // These are metadata examples; recording downloads retain their existing availability.
  const recordingBytes = Number(status.metrics.at(-1)?.recordingBytes ?? 0);
  const recordingCount = status.recordings.length;
  source(HOST, `${status.projectDirectory}/.console/runs/${status.runId}`, [], status.recordings.flatMap((output, index) => [
    { path: pathBytes(output.path), kind: ADDED, after: current(
      Math.floor(recordingBytes / recordingCount) + (index === recordingCount - 1 ? recordingBytes % recordingCount : 0),
    ) },
    { path: pathBytes(`${output.path}.report.pbtxt`), kind: ADDED, after: current(172) },
  ]));
  source(LAYER, digest(`${status.runId}:object-detector`), [`${project}/object-detector:/`], [
    { path: pathBytes('/var/cache/inference/detector.engine'), kind: ADDED },
    { path: pathBytes('/etc/ld.so.cache'), kind: MODIFIED },
    { path: pathBytes('/tmp/model-download.part'), kind: DELETED },
  ]);
  source(LAYER, digest(`${status.runId}:motion-planner`), [`${project}/motion-planner:/`], [
    { path: pathBytes('/tmp/planner'), kind: ADDED },
    { path: pathBytes('/var/log/planner.log'), kind: MODIFIED },
  ]);

  return wire.FileChanges.create({
    state: ReportState.REPORT_STATE_COMPLETE, downloadable: true, preview,
    coverageNotes: CATEGORIES.flatMap(category => category.notes),
    report: filesystem.FileChangeReference.create({ checkpoint: 'filesystem/report.pb', categories: CATEGORIES.map(({ category, manifest, notes }) => {
      const changes = preview.filter(change => change.category === category);
      return filesystem.CategorySummary.create({
        category, manifest, coverageComplete: notes.length === 0, coverageNoteCount: notes.length,
        counts: filesystem.ChangeCounts.create({
          added: changes.filter(change => change.kind === ADDED).length,
          modified: changes.filter(change => change.kind === MODIFIED).length,
          deleted: changes.filter(change => change.kind === DELETED).length,
        }),
      });
    }) }),
  });
}

/** Export the complete demo preview in the server's JSONL format, including coverage and completion. */
export function demoFileChangesJsonl(view: doramcap.console.v1.IFileChanges) {
  const lines: object[] = [];
  for (const summary of view.report!.categories!) {
    lines.push({ type: 'category', ...filesystem.CategorySummary.toObject(filesystem.CategorySummary.create(summary), JSON_OPTIONS) });
    for (const note of CATEGORIES.find(category => category.category === summary.category)!.notes)
      lines.push({ type: 'coverage', category: Category[summary.category!], note });
    for (const change of view.preview!.filter(change => change.category === summary.category)) {
      const record = filesystem.FileChange.toObject(filesystem.FileChange.create(change), JSON_OPTIONS);
      for (const side of ['before', 'after'] as const) if (record[side]?.symlink != null)
        record[side].symlinkDisplay = displayPath(change[side]!.symlink!);
      lines.push({ type: 'change', ...record, pathDisplay: displayPath(change.path!) });
    }
  }
  lines.push({ type: 'complete', changes: String(view.preview!.length), coverageComplete: view.report!.categories!.every(category => category.coverageComplete) });
  return lines.map(line => JSON.stringify(line) + '\n').join('');
}
