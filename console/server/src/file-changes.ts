import { createReadStream, closeSync, fstatSync } from 'node:fs';
import { realpath } from 'node:fs/promises';
import Long from 'long';
import { isAbsolute, join } from 'node:path';
import type { ReadStream } from 'node:fs';
import { capabilities, wire, browserError } from './native.js';
import { doramcap } from './generated/console.js';
import { displayPath } from './generated/transport.js';

export const filesystem = doramcap.filesystem.v1;
const CATEGORIES = [filesystem.StorageCategory.STORAGE_CATEGORY_HOST_BIND, filesystem.StorageCategory.STORAGE_CATEGORY_CONTAINER_LAYER, filesystem.StorageCategory.STORAGE_CATEGORY_NAMED_VOLUME];
const CHANGE_KINDS = [filesystem.ChangeKind.CHANGE_KIND_ADDED, filesystem.ChangeKind.CHANGE_KIND_MODIFIED, filesystem.ChangeKind.CHANGE_KIND_DELETED];
const CHECKPOINT = 'filesystem/report.pb';
const PRIVATE_DIRECTORIES = ['filesystem', 'overlays', 'kept'];
type Reference = doramcap.filesystem.v1.IFileChangeReference;
type Summary = doramcap.filesystem.v1.ICategorySummary;
type View = typeof wire.FileChanges.prototype;
// Fits the 2 MiB saved-detail and 4 MiB transport budgets, including complete records.
export const FILE_CHANGE_PREVIEW_BYTES = 8 * 1024;
const JSON_OPTIONS = { longs: String, bytes: String, enums: String, defaults: true } as const;
const integer = (value: unknown) => BigInt(String(value ?? 0));
const count = (summary: Summary) => integer(summary.counts?.added) + integer(summary.counts?.modified) + integer(summary.counts?.deleted);
const line = (value: unknown) => Buffer.from(`${JSON.stringify(value)}\n`);

/** Open only regular files inside this run; traversal and symlink escapes fail closed. */
function descriptor(runRoot: string, path: string) {
  if (!path || isAbsolute(path) || path.split('/').some(part => !part || part === '..' || part === '.'))
    throw new Error('Invalid file-change reference');
  return capabilities.openFile(runRoot, path);
}
function open(runRoot: string, path: string, signal?: AbortSignal): ReadStream {
  return createReadStream('', { fd: descriptor(runRoot, path), autoClose: true, signal });
}

/** Read one generated checkpoint through the same run-confined opener as manifests. */
async function checkpoint(runRoot: string, path: string, signal?: AbortSignal) {
  const chunks: Buffer[] = [];
  for await (const chunk of open(runRoot, path, signal)) chunks.push(Buffer.from(chunk));
  return filesystem.FileChangeReport.decode(Buffer.concat(chunks));
}

/** Read the published checkpoint and verify its bounded reference; missing publication is unavailable. */
async function report(runRoot: string, reference: Reference, signal?: AbortSignal) {
  const saved = await checkpoint(runRoot, reference.checkpoint!, signal);
  if (saved.state !== filesystem.ReportState.REPORT_STATE_COMPLETE || !saved.categories.length || saved.categories.length !== reference.categories?.length)
    throw new Error('File-change category mismatch');
  const seen = new Set<number>();
  for (const [index, category] of saved.categories.entries()) {
    const summary = category.summary;
    if (!summary || !summary.counts || !CATEGORIES.includes(summary.category!) || seen.has(summary.category!))
      throw new Error('Invalid file-change category');
    seen.add(summary.category!);
    if (integer(summary.coverageNoteCount) !== BigInt((category.coverageNotes ?? []).length) || !Buffer.from(filesystem.CategorySummary.encode(summary).finish()).equals(
      Buffer.from(filesystem.CategorySummary.encode(reference.categories![index]).finish()),
    )) throw new Error('File-change checkpoint differs from saved reference');
  }
  return saved;
}

/** Decode delimited protobuf incrementally; EOF inside a length or record is an error. */
async function* changes(source: ReadStream) {
  let pending = Buffer.alloc(0);
  try {
    for await (const chunk of source) {
      pending = pending.length ? Buffer.concat([pending, chunk]) : Buffer.from(chunk);
      let offset = 0;
      while (offset < pending.length) {
        let length = 0, prefix = 0;
        for (; prefix < 5 && offset + prefix < pending.length; prefix++) {
          const byte = pending[offset + prefix];
          if (prefix === 4 && byte > 15) throw new Error('Invalid file-change record length');
          length += (byte & 0x7f) * 2 ** (prefix * 7);
          if (byte < 128) break;
        }
        if (prefix === 5) throw new Error('Invalid file-change record length');
        if (offset + prefix === pending.length || pending.length - offset - prefix - 1 < length) break;
        const start = offset + prefix + 1;
        const row = filesystem.FileChange.decode(pending.subarray(start, start + length));
        if (!CATEGORIES.includes(row.category) || !CHANGE_KINDS.includes(row.kind))
          throw new Error('Invalid file-change record');
        yield row;
        offset = start + length;
      }
      pending = pending.subarray(offset);
    }
    if (pending.length) throw new Error('Truncated file-change manifest');
  } finally {
    source.destroy();
    if (!source.closed) await new Promise<void>(resolve => source.once('close', () => resolve()));
  }
}

/** Traverse all saved records and check each kind's count; whole-record truncation also fails. */
async function* categoryChanges(runRoot: string, summary: Summary, signal?: AbortSignal) {
  const counts = [0n, 0n, 0n];
  for await (const row of changes(open(runRoot, summary.manifest!, signal))) {
    if (row.category !== summary.category) throw new Error('File-change storage category mismatch');
    counts[row.kind - 1]++;
    yield row;
  }
  const expected = [summary.counts?.added, summary.counts?.modified, summary.counts?.deleted].map(integer);
  if (counts.some((n, index) => n !== expected[index])) throw new Error('File-change manifest count mismatch');
}

/** Save a whole-record preview shared across categories; export validates records beyond its display budget.
 * Memory includes the checkpoint's coverage notes, one whole record and the bounded preview.
 */
export async function fileChangePreview(runRoot: string, reference?: Reference | null, state = filesystem.ReportState.REPORT_STATE_UNAVAILABLE): Promise<View | null> {
  if (!reference && state === filesystem.ReportState.REPORT_STATE_CHECKPOINT_UNKNOWN) {
    // Later native report writes can fail after capture has already been published.
    try {
      const saved = await checkpoint(runRoot, CHECKPOINT);
      if (!saved.categories.length || saved.categories.some(category => !category.summary?.counts))
        throw new Error('File-change checkpoint has no category summary');
      reference = { checkpoint: CHECKPOINT, categories: saved.categories.map(category => category.summary!) };
    } catch {
      return wire.FileChanges.create({ state: filesystem.ReportState.REPORT_STATE_FAILED, error: 'File changes unavailable: no valid published checkpoint.' });
    }
  }
  if (!reference) return state === filesystem.ReportState.REPORT_STATE_UNAVAILABLE ? null
    : wire.FileChanges.create({ state: filesystem.ReportState.REPORT_STATE_FAILED, error: 'File changes were not collected; inspect the run diagnostics.' });
  const view = wire.FileChanges.create({ state: filesystem.ReportState.REPORT_STATE_COMPLETE, report: reference });
  try {
    const saved = await report(runRoot, reference);
    if (saved.writePolicy === filesystem.WritePolicy.WRITE_POLICY_KEEP) {
      view.keptDirectory = join(runRoot, 'kept');
    }
    for (const category of saved.categories) {
      closeSync(descriptor(runRoot, category.summary!.manifest!));
      for (const note of category.coverageNotes ?? []) {
        if (view.notesIncomplete) break;
        view.coverageNotes.push(note);
        if (wire.FileChanges.encode(view).finish().length > FILE_CHANGE_PREVIEW_BYTES) {
          view.coverageNotes.pop(); view.notesIncomplete = true;
        }
      }
    }
    // Take one record from each category in turn so every category with changes shares the budget.
    const canonicalRoot = await realpath(runRoot);
    let offset = 0n;
    const streams = saved.categories.map(category => {
      const stream = { changes: categoryChanges(runRoot, category.summary!), position: offset };
      offset += count(category.summary!);
      return stream;
    });
    try {
      let active = streams;
      while (active.length && !view.previewIncomplete) {
        const remaining: typeof streams = [];
        for (const stream of active) {
          const next = await stream.changes.next();
          if (next.done) continue;
          const position = Long.fromString(String(stream.position++), true);
          view.preview.push(next.value);
          view.previewPositions.push(position);
          try {
            closeSync(openChange(runRoot, canonicalRoot, saved, next.value).fd);
            view.downloadablePositions.push(position);
          } catch { /* This entry has no downloadable regular file. */ }
          if (wire.FileChanges.encode(view).finish().length > FILE_CHANGE_PREVIEW_BYTES) {
            view.preview.pop(); view.previewPositions.pop();
            view.downloadablePositions = view.downloadablePositions.filter(value => String(value) !== String(position));
            view.previewIncomplete = true;
            break;
          }
          remaining.push(stream);
        }
        active = remaining;
      }
    } finally {
      // Close manifests left unread; their tails are validated by export.
      await Promise.all(streams.map(stream => stream.changes.return(undefined)));
    }
    view.downloadable = true;
  } catch (error) {
    view.state = filesystem.ReportState.REPORT_STATE_FAILED;
    view.error = browserError(`File changes unavailable: ${(error as Error).message}`);
    view.previewIncomplete = true;
  }
  while (wire.FileChanges.encode(view).finish().length > FILE_CHANGE_PREVIEW_BYTES) {
    if (view.preview.length) {
      view.preview.pop(); const position = view.previewPositions.pop();
      view.downloadablePositions = view.downloadablePositions.filter(value => String(value) !== String(position));
      view.previewIncomplete = true;
    }
    else if (view.coverageNotes.length) { view.coverageNotes.pop(); view.notesIncomplete = true; }
    else return wire.FileChanges.create({ state: filesystem.ReportState.REPORT_STATE_FAILED, error: 'File-change reference exceeds the display budget.' });
  }
  return view;
}

/** Recheck saved references without reading manifests on every live-status request. */
export function fileChangeAvailability(runRoot: string, view?: doramcap.console.v1.IFileChanges | null) {
  if (!view?.report || !view.downloadable) return view;
  try {
    for (const path of [view.report.checkpoint, ...(view.report.categories ?? []).map(c => c.manifest)]) {
      closeSync(descriptor(runRoot, path!));
    }
  } catch {
    view.downloadable = false;
    view.state = filesystem.ReportState.REPORT_STATE_FAILED;
    view.error = 'File-change checkpoint or manifest is unavailable.';
  }
  return view;
}

/** Stream category summaries, full coverage notes and changes; completion appears only after verified EOF. */
export async function* exportFileChanges(runRoot: string, reference: Reference, signal?: AbortSignal) {
  const saved = await report(runRoot, reference, signal);
  let total = 0n;
  for (const category of saved.categories) {
    const summary = category.summary!;
    yield line({ type: 'category', ...filesystem.CategorySummary.toObject(filesystem.CategorySummary.create(summary), JSON_OPTIONS) });
    for (const note of category.coverageNotes ?? [])
      yield line({ type: 'coverage', category: filesystem.StorageCategory[summary.category!], note });
    for await (const change of categoryChanges(runRoot, summary, signal)) {
      const record = filesystem.FileChange.toObject(change, JSON_OPTIONS);
      for (const side of ['before', 'after']) if (record[side]?.symlink != null)
        record[side].symlinkDisplay = displayPath(Buffer.from(record[side].symlink, 'base64'));
      yield line({ type: 'change', position: (total++).toString(), ...record, pathDisplay: displayPath(change.path) });
    }
  }
  yield line({ type: 'complete', changes: total.toString(), coverageComplete: saved.categories.every(c => c.summary!.coverageComplete) });
}

/** Resolve a zero-based position after validating every manifest through EOF; memory holds one record.
 * Paths and roots come only from the saved checkpoint. Symlinks and private records fail closed.
 */
export async function openChangedFile(runRoot: string, reference: Reference, position: bigint, signal: AbortSignal) {
  if (position < 0n) throw new Error('Missing');
  const saved = await report(runRoot, reference, signal);
  let index = 0n;
  let selected: typeof filesystem.FileChange.prototype | undefined;
  for (const category of saved.categories) {
    for await (const change of categoryChanges(runRoot, category.summary!, signal)) {
      if (index++ === position) selected = change;
    }
  }
  signal.throwIfAborted();
  if (!selected) throw new Error('Missing');
  return openChange(runRoot, await realpath(runRoot), saved, selected);
}

function relativeBytes(path: Uint8Array) {
  const bytes = Buffer.from(path);
  if (!bytes.length || bytes.includes(0) || bytes.toString('latin1').split('/').some(part => !part || part === '.' || part === '..'))
    throw new Error('Missing');
  return bytes;
}
function beneath(path: Buffer, parent: Buffer) {
  return !parent.length || path.equals(parent) || path.subarray(0, parent.length + 1).equals(Buffer.concat([parent, Buffer.from('/')]));
}
function openChange(runRoot: string, canonicalRoot: string, saved: typeof filesystem.FileChangeReport.prototype, change: typeof filesystem.FileChange.prototype) {
  const REGULAR_FILE = 0o100000, FILE_TYPE = 0o170000;
  if (change.category !== filesystem.StorageCategory.STORAGE_CATEGORY_HOST_BIND || change.kind === filesystem.ChangeKind.CHANGE_KIND_DELETED
      || !change.after || ((change.after.mode ?? 0) & FILE_TYPE) !== REGULAR_FILE) throw new Error('Missing');
  const path = relativeBytes(change.path);
  const mappings = saved.overlaySources.filter(source => Buffer.from(source.source ?? []).equals(Buffer.from(change.source)));
  let relative: Buffer;
  if (mappings.length) {
    if (mappings.length !== 1 || saved.writePolicy !== filesystem.WritePolicy.WRITE_POLICY_KEEP) throw new Error('Missing');
    closeSync(descriptor(runRoot, 'overlays/results.json'));
    relative = Buffer.concat([Buffer.from(`kept/${mappings[0].index}/`), path]);
  } else {
    if (change.source !== canonicalRoot && !change.source.startsWith(`${canonicalRoot}/`)) throw new Error('Missing');
    const source = Buffer.from(change.source.slice(canonicalRoot.length + 1));
    relative = relativeBytes(source.length ? Buffer.concat([source, Buffer.from('/'), path]) : path);
    const excluded = [...PRIVATE_DIRECTORIES, 'console-report.json'].map(value => Buffer.from(value));
    excluded.push(...saved.excluded.map(value => Buffer.from(value)));
    if (excluded.some(parent => beneath(relative, parent))) throw new Error('Missing');
  }
  const fd = capabilities.openFile(runRoot, relative);
  try {
    const metadata = fstatSync(fd, { bigint: true });
    if (!metadata.isFile() || metadata.nlink !== 1n || metadata.size !== integer(change.after.size)) throw new Error('Missing');
    return { fd, filename: path.subarray(path.lastIndexOf(47) + 1) };
  } catch (error) { closeSync(fd); throw error; }
}
