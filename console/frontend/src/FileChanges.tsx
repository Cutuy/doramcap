import { useConsole } from './context';
import { artifactFilename, displayPath, FILE_CHANGES_ARTIFACT } from './generated/transport';
import type { Snapshot } from './api';
import { doramcap } from './generated/console';

const { StorageCategory, ChangeKind } = doramcap.filesystem.v1;
const categoryLabel = (category: number) => category === StorageCategory.STORAGE_CATEGORY_HOST_BIND ? 'Host folders'
  : category === StorageCategory.STORAGE_CATEGORY_CONTAINER_LAYER ? 'Container layers' : 'Named volumes';
const integer = (value: unknown) => BigInt(String(value ?? 0));

/** One bounded saved preview, shared by the current result and History. Paths are plain text. */
export function FileChanges({ snapshot }: { snapshot: Snapshot }) {
  const { client } = useConsole();
  const view = snapshot.fileChanges;
  const summaries = view?.report?.categories ?? [];
  const total = summaries.reduce((sum, c) => sum + integer(c.counts?.added) + integer(c.counts?.modified) + integer(c.counts?.deleted), 0n);
  return <section className="panel file-changes">
    <div className="panel-heading"><h2>File changes</h2>{view?.downloadable && <a className="button secondary" href={client.artifactUrl(snapshot.runId, FILE_CHANGES_ARTIFACT)} download={artifactFilename(FILE_CHANGES_ARTIFACT)}>Download file changes</a>}</div>
    {!view && <p className="unavailable">File changes unavailable for this run.</p>}
    {view?.error && <p className="notice failure-notice" role="alert">{view.error}</p>}
    {summaries.length > 0 && <>
      <div className="file-change-counts">{summaries.map(summary => <div key={summary.category}>
        <h3>{categoryLabel(summary.category!)}</h3>
        <p>{String(summary.counts?.added ?? 0)} added · {String(summary.counts?.modified ?? 0)} modified · {String(summary.counts?.deleted ?? 0)} deleted</p>
        <p className="muted">{summary.coverageComplete ? 'Coverage complete within declared sources' : 'Incomplete observation'} · {String(summary.coverageNoteCount ?? 0)} coverage notes</p>
      </div>)}</div>
      {(view!.coverageNotes ?? []).map((note, index) => <p className="muted break" key={index}>{note}</p>)}
      {view!.notesIncomplete && <p className="muted">More coverage notes are included in the download.</p>}
      <p className="muted">Preview: {view!.preview.length} of {total.toString()} collected changes{view!.previewIncomplete ? ' · Preview incomplete' : ''}.</p>
      {view!.preview.length > 0 && <div className="file-change-list" tabIndex={0} aria-label="File change preview">{view!.preview.map((change, index) => <article key={index}>
        <div><strong>{ChangeKind[change.kind!].replace(/^CHANGE_KIND_/, '').toLowerCase()}</strong> · {categoryLabel(change.category!)} · <span className="mono break">{change.source}</span></div>
        <p className="mono break">{displayPath(change.path!) || '(source root)'}</p>
        {(change.consumers ?? []).length > 0 && <p className="muted break">{change.consumers!.join(', ')}</p>}
        {[['Before', change.before], ['After', change.after]].map(([label, metadata]) => {
          const value = metadata as doramcap.filesystem.v1.IFileMetadata | null;
          return <p className="muted break" key={String(label)}>{String(label)}: {value
            ? `${String(value.size ?? 0)} bytes · mode ${(value.mode ?? 0).toString(8)} · uid ${value.uid ?? 0} · gid ${value.gid ?? 0} · mtime ${String(value.mtimeSeconds ?? 0)} s + ${String(value.mtimeNanoseconds ?? 0)} ns · ctime ${String(value.ctimeSeconds ?? 0)} s + ${String(value.ctimeNanoseconds ?? 0)} ns${value.symlink != null ? ` · symlink ${displayPath(value.symlink)}` : ''}`
            : 'metadata unavailable'}</p>;
        })}
      </article>)}</div>}
      {total === 0n && !view!.error && <p>No changes observed.</p>}
    </>}
  </section>;
}
