import { useState } from 'react';
import { doramcap } from './generated/console';
import { WritePolicySetting, writePolicyLabel } from './WritePolicy';
import { ActionButton } from './ui/ActionButton';

const { SourceKind } = doramcap.filesystem.v1;
type Folder = doramcap.filesystem.v1.IWritableFolder;

/** Read-only handling of reviewed host folders; Copy only writes the clipboard. */
export function WritableFolders({ folders, policy, kept }: { folders: Folder[]; policy?: number | null; kept?: string | null }) {
  const [copied, setCopied] = useState<{ source: string; success: boolean } | null>(null);
  const settings = folders.filter(folder => folder.kind === SourceKind.SOURCE_KIND_OVERLAY).map(folder => folder.source ?? '');
  const copy = async (source: string) => {
    try { await navigator.clipboard.writeText(`passthrough_folders: ${JSON.stringify(source)}`); setCopied({ source, success: true }); }
    catch { setCopied({ source, success: false }); }
  };
  return <section className="panel writable-folders" aria-label="Writable folders">
    <h2>Writable folders</h2>
    {folders.length ? <table><thead><tr><th scope="col">Host folder</th><th scope="col">App</th><th scope="col">During the run</th><th scope="col">After the run</th></tr></thead><tbody>
      {folders.map(folder => <tr key={folder.source}>
        <td data-label="Host folder" className="mono break">{folder.source}</td>
        <td data-label="App" className="break">{folder.apps?.join(', ')}</td>
        <td data-label="During the run">{folder.kind === SourceKind.SOURCE_KIND_OVERLAY ? 'Overlay' : folder.kind === SourceKind.SOURCE_KIND_PASSTHROUGH ? 'Passed through' : `Not tracked: ${folder.reason?.replace(/^not tracked:\s*/i, '') || 'handling unavailable'}`}</td>
        <td data-label="After the run">{folder.kind === SourceKind.SOURCE_KIND_OVERLAY ? writePolicyLabel(policy) : [SourceKind.SOURCE_KIND_PASSTHROUGH, SourceKind.SOURCE_KIND_UNTRACKED, SourceKind.SOURCE_KIND_HOST_SHARED_MEMORY].includes(folder.kind ?? SourceKind.SOURCE_KIND_UNSPECIFIED) ? 'Already on the host; no change list' : 'Unavailable'}</td>
      </tr>)}
    </tbody></table> : <p>No writable host folders.</p>}
    <WritePolicySetting policy={policy} kept={kept}/>
    {settings.length > 0 && <><h3>Pass a folder through</h3>
    {settings.map(source => <div className="folder-setting" key={source}>
      <code className="mono break">{`passthrough_folders: ${JSON.stringify(source)}`}</code>
      <ActionButton variant="secondary" aria-label={`Copy passthrough setting for ${source}`} onClick={() => void copy(source)}>{copied?.source === source ? copied.success ? 'Copied' : 'Copy failed' : 'Copy'}</ActionButton>
    </div>)}</>}
  </section>;
}
