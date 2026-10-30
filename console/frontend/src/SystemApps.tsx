import type { SystemApp } from './api';

export function SystemApps({ apps, busy, toggle }: { apps: SystemApp[]; busy: boolean; toggle: (id: string) => void }) {
  return <section className="panel"><h2>System apps</h2>{apps.length ? <fieldset disabled={busy} aria-label="System apps">{apps.map(app => <label className="checkbox" key={app.id}><input type="checkbox" checked={app.selected} onChange={() => toggle(app.id)}/>{app.label}</label>)}</fieldset> : <p className="unavailable">None configured</p>}</section>;
}
