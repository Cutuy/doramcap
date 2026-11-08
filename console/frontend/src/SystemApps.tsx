import type { SystemApp } from './api';
import { Select } from './ui/Select';

export function SystemApps({ apps, busy, toggle, chooseProfile }: { apps: SystemApp[]; busy: boolean; toggle: (id: string) => void; chooseProfile: (id: string, profile: string) => void }) {
  return <section className="panel"><h2>System apps</h2>{apps.length ? <fieldset disabled={busy} aria-label="System apps">{apps.map(app => <div key={app.id}><label className="checkbox"><input type="checkbox" checked={app.selected} disabled={app.required} onChange={() => toggle(app.id)}/>{app.label}{app.required && <span className="muted"> · Required</span>}</label>{app.profiles.length > 1 && <Select className="app-profile" aria-label={`${app.label} profile`} value={app.profile} disabled={busy || !app.selected} onChange={profile => chooseProfile(app.id, profile)} options={app.profiles.map(profile => ({ value: profile.id, label: profile.label }))}/>}</div>)}</fieldset> : <p className="unavailable">None configured</p>}</section>;
}
