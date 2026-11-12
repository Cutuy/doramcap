import { useState } from 'react';
import { useConsole } from './context';
import { artifactFilename, displayPath, FILE_CHANGES_ARTIFACT } from './generated/transport';
import type { Snapshot } from './api';
import { doramcap } from './generated/console';
import { bytesLabel } from './RunMetrics';
import { ActionButton } from './ui/ActionButton';

const { StorageCategory, ChangeKind } = doramcap.filesystem.v1;
type Change = doramcap.filesystem.v1.IFileChange;
type Metadata = doramcap.filesystem.v1.IFileMetadata | null | undefined;

const categoryLabel = (category: number) => category === StorageCategory.STORAGE_CATEGORY_HOST_BIND ? 'Host folders'
  : category === StorageCategory.STORAGE_CATEGORY_CONTAINER_LAYER ? 'Container layers' : 'Named volumes';
const sourceLabel = (category: number) => category === StorageCategory.STORAGE_CATEGORY_HOST_BIND ? 'Host folder'
  : category === StorageCategory.STORAGE_CATEGORY_CONTAINER_LAYER ? 'Container layer' : 'Named volume';
const KINDS = [
  { kind: ChangeKind.CHANGE_KIND_ADDED, label: 'Added', mark: '+', className: 'added' },
  { kind: ChangeKind.CHANGE_KIND_MODIFIED, label: 'Modified', mark: '~', className: 'modified' },
  { kind: ChangeKind.CHANGE_KIND_DELETED, label: 'Deleted', mark: '−', className: 'deleted' },
] as const;
const kindOf = (kind: number | null | undefined) => KINDS.find(entry => entry.kind === kind);
const integer = (value: unknown) => BigInt(String(value ?? 0));

const FILE_TYPE_MASK = 0o170000;
const PERMISSION_MASK = 0o7777;
const FILE_TYPES: Record<number, string> = { 0o040000: 'directory', 0o120000: 'symlink', 0o100000: 'file' };
const NANOS_PER_SECOND = 1_000_000_000n;
const fileType = (metadata: NonNullable<Metadata>) => FILE_TYPES[(metadata.mode ?? 0) & FILE_TYPE_MASK] ?? 'special file';
const permissions = (metadata: NonNullable<Metadata>) => ((metadata.mode ?? 0) & PERMISSION_MASK).toString(8).padStart(4, '0');
const owner = (metadata: NonNullable<Metadata>) => `${metadata.uid ?? 0}:${metadata.gid ?? 0}`;
const target = (metadata: NonNullable<Metadata>) => metadata.symlink != null ? displayPath(metadata.symlink) : null;
const nanos = (seconds: unknown, nanoseconds: unknown) => integer(seconds) * NANOS_PER_SECOND + integer(nanoseconds);
/** Signed elapsed time between two stat times, in the largest whole unit that keeps it readable. */
const elapsedLabel = (delta: bigint) => {
  const sign = delta < 0n ? '−' : '+';
  const magnitude = delta < 0n ? -delta : delta;
  const seconds = Number(magnitude) / 1e9;
  if (magnitude < 1_000_000n) return `${sign}${magnitude} ns`;
  if (seconds < 1) return `${sign}${(seconds * 1000).toFixed(0)} ms`;
  if (seconds < 60) return `${sign}${seconds.toFixed(seconds < 10 ? 1 : 0)} s`;
  if (seconds < 3600) return `${sign}${(seconds / 60).toFixed(0)} min`;
  return `${sign}${(seconds / 3600).toFixed(1)} h`;
};
/** UTC stat time with nanoseconds; falls back to raw seconds outside the Date range. */
const timeLabel = (seconds: unknown, nanoseconds: unknown) => {
  const date = new Date(Number(integer(seconds)) * 1000);
  const fraction = String(nanoseconds ?? 0).padStart(9, '0');
  return Number.isNaN(date.getTime()) ? `${String(seconds ?? 0)} s + ${String(nanoseconds ?? 0)} ns`
    : `${date.toISOString().slice(0, 19).replace('T', ' ')}.${fraction} UTC`;
};
const sizeLabel = (metadata: NonNullable<Metadata>) => fileType(metadata) === 'file' ? bytesLabel(metadata.size) : fileType(metadata);

/** Short size column and the facts that distinguish this change, with changed identity fields flagged. */
function summary(change: Change): { size: string; facts: { text: string; changed?: boolean }[] } {
  const { before, after } = change;
  if (change.kind === ChangeKind.CHANGE_KIND_ADDED && after) {
    return { size: sizeLabel(after), facts: [{ text: `${permissions(after)} · ${owner(after)}` }, ...(target(after) != null ? [{ text: `→ ${target(after)}` }] : [])] };
  }
  if (change.kind === ChangeKind.CHANGE_KIND_DELETED && before) return { size: sizeLabel(before), facts: [{ text: `was ${permissions(before)} · ${owner(before)}` }] };
  if (change.kind === ChangeKind.CHANGE_KIND_MODIFIED && before && after) {
    const facts: { text: string; changed?: boolean }[] = [];
    if (fileType(before) !== fileType(after)) facts.push({ text: `${fileType(before)} → ${fileType(after)}`, changed: true });
    if (permissions(before) !== permissions(after)) facts.push({ text: `mode ${permissions(before)} → ${permissions(after)}`, changed: true });
    if (owner(before) !== owner(after)) facts.push({ text: `owner ${owner(before)} → ${owner(after)}`, changed: true });
    if (target(before) !== target(after)) facts.push({ text: `target ${target(before) ?? '—'} → ${target(after) ?? '—'}`, changed: true });
    const mtime = nanos(after.mtimeSeconds, after.mtimeNanoseconds) - nanos(before.mtimeSeconds, before.mtimeNanoseconds);
    if (mtime !== 0n) facts.push({ text: `mtime ${elapsedLabel(mtime)}` });
    if (facts.length === 0) facts.push({ text: 'ctime only' });
    const size = integer(before.size) !== integer(after.size) && fileType(after) === 'file' ? `${bytesLabel(before.size)} → ${bytesLabel(after.size)}` : sizeLabel(after);
    return { size, facts };
  }
  return { size: '—', facts: [{ text: 'metadata unavailable' }] };
}

/** Every stored stat field, before and after, for the full-metadata view. */
function MetadataTable({ change }: { change: Change }) {
  const sides = [change.before, change.after];
  const rows: [string, (metadata: NonNullable<Metadata>) => string][] = [
    ['Type', fileType], ['Size', metadata => `${String(metadata.size ?? 0)} bytes`], ['Mode', permissions], ['Owner (uid:gid)', owner],
    ['Modified (mtime)', metadata => timeLabel(metadata.mtimeSeconds, metadata.mtimeNanoseconds)],
    ['Changed (ctime)', metadata => timeLabel(metadata.ctimeSeconds, metadata.ctimeNanoseconds)],
    ['Symlink target', metadata => target(metadata) ?? '—'],
  ];
  return <dl className="file-change-metadata">
    <dt/><dd className="muted">Before</dd><dd className="muted">After</dd>
    {rows.map(([label, value]) => {
      const [first, second] = sides.map(side => side ? value(side) : 'unavailable');
      return <div className={first !== second && change.before && change.after ? 'changed' : ''} key={label}><dt>{label}</dt><dd>{first}</dd><dd>{second}</dd></div>;
    })}
  </dl>;
}

/** One bounded saved preview, shared by the current result and History. Paths are plain text. */
export function FileChanges({ snapshot }: { snapshot: Snapshot }) {
  const { client } = useConsole();
  const [filter, setFilter] = useState<number | null>(null);
  const [showMetadata, setShowMetadata] = useState(false);
  const view = snapshot.fileChanges;
  const summaries = view?.report?.categories ?? [];
  const total = summaries.reduce((sum, c) => sum + integer(c.counts?.added) + integer(c.counts?.modified) + integer(c.counts?.deleted), 0n);
  const preview = view?.preview ?? [];
  const shown = preview.filter(change => filter == null || change.kind === filter);
  // Group by source in first-seen order so each source header appears once.
  const groups = new Map<string, Change[]>();
  for (const change of shown) {
    const key = `${change.category}\0${change.source}`;
    groups.set(key, [...(groups.get(key) ?? []), change]);
  }
  return <section className="panel file-changes">
    <div className="panel-heading"><h2>File changes</h2>{view?.downloadable && <a className="button secondary" href={client.artifactUrl(snapshot.runId, FILE_CHANGES_ARTIFACT)} download={artifactFilename(FILE_CHANGES_ARTIFACT)}>Download file changes</a>}</div>
    {!view && <p className="unavailable">File changes unavailable for this run.</p>}
    {view?.error && <p className="notice failure-notice" role="alert">{view.error}</p>}
    {summaries.length > 0 && <>
      <div className="file-change-counts">{summaries.map(summary => {
        const counts = [integer(summary.counts?.added), integer(summary.counts?.modified), integer(summary.counts?.deleted)];
        const notes = integer(summary.coverageNoteCount);
        return <div className={`file-change-tile${summary.coverageComplete ? '' : ' incomplete'}`} key={summary.category}>
          <h3>{categoryLabel(summary.category!)}</h3>
          {counts.some(count => count > 0n)
            ? <p className="file-change-tally"><span aria-hidden="true">{KINDS.map((entry, index) => <span className={counts[index] > 0n ? entry.className : 'muted'} key={entry.kind}>{entry.mark}{counts[index].toString()}</span>)}</span><span className="sr-only">{`${counts[0]} added · ${counts[1]} modified · ${counts[2]} deleted`}</span></p>
            : <p className="file-change-tally muted">No changes</p>}
          <p className="file-change-coverage">{summary.coverageComplete ? 'Coverage complete' : 'Incomplete observation'}{notes > 0n ? ` · ${notes} coverage ${notes === 1n ? 'note' : 'notes'}` : ''}</p>
        </div>;
      })}</div>
      {(view!.coverageNotes ?? []).length > 0 && <ul className="file-change-notes" aria-label="Coverage notes">
        {view!.coverageNotes.map((note, index) => <li className="break" key={index}>{note}</li>)}
        {view!.notesIncomplete && <li>More coverage notes are included in the download.</li>}
      </ul>}
      {preview.length > 0 && <>
        <div className="file-change-toolbar">
          <div className="file-change-filters" role="group" aria-label="Filter by change kind">
            <button type="button" className="button secondary" aria-pressed={filter == null} onClick={() => setFilter(null)}>All {preview.length}</button>
            {KINDS.map(entry => {
              const count = preview.filter(change => change.kind === entry.kind).length;
              return count > 0 && <button type="button" className="button secondary" aria-pressed={filter === entry.kind} onClick={() => setFilter(entry.kind)} key={entry.kind}>{entry.label} {count}</button>;
            })}
          </div>
          <ActionButton type="button" variant="secondary" onClick={() => setShowMetadata(!showMetadata)}>{showMetadata ? 'Hide full metadata' : 'Show full metadata'}</ActionButton>
        </div>
        <div className="file-change-list" tabIndex={0} aria-label="File change preview">
          <table>
            <thead className="sr-only"><tr><th>Kind</th><th>Path</th><th>Size</th><th>Details</th></tr></thead>
            {[...groups.values()].map(changes => {
              const first = changes[0];
              const consumers = first.consumers ?? [];
              return <tbody key={`${first.category}\0${first.source}`}>
                <tr className="file-change-source"><th colSpan={4} scope="rowgroup">
                  <span className="mono break">{first.source}</span>
                  <span className="muted">{sourceLabel(first.category!)}{consumers.length > 0 ? ` · ${consumers.join(', ')}` : ''} · {changes.length} {changes.length === 1 ? 'change' : 'changes'}</span>
                </th></tr>
                {changes.map((change, index) => {
                  const kind = kindOf(change.kind);
                  const { size, facts } = summary(change);
                  const path = displayPath(change.path!);
                  return [<tr className="file-change-row" key={index}>
                    <td><span className={`file-change-kind ${kind?.className ?? ''}`} title={kind?.label ?? 'Unknown'}>{kind?.mark ?? '?'}<span className="sr-only"> {kind?.label ?? 'Unknown'}</span></span></td>
                    <td className="mono break">{path || <span className="muted">(source root)</span>}</td>
                    <td className="muted">{size}</td>
                    <td>{facts.map((fact, factIndex) => <span className={fact.changed ? 'file-change-fact changed' : 'file-change-fact muted'} key={factIndex}>{fact.text}</span>)}</td>
                  </tr>, showMetadata && (change.before || change.after) && <tr className="file-change-detail" key={`${index}-metadata`}><td/><td colSpan={3}><MetadataTable change={change}/></td></tr>];
                })}
              </tbody>;
            })}
          </table>
        </div>
      </>}
      {(total > 0n || view!.previewIncomplete) && <p className="muted file-change-shown">Showing {shown.length} of {total.toString()} changes{view!.previewIncomplete ? ' · Preview incomplete; download for the full list' : ''}</p>}
      {total === 0n && !view!.error && <p>No changes observed.</p>}
    </>}
  </section>;
}
