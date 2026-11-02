import type { SystemApp } from './api';

export function SystemApps({ apps, busy, toggle, chooseProfile }: { apps: SystemApp[]; busy: boolean; toggle: (id: string) => void; chooseProfile: (id: string, profile: string) => void }) {
  return <section className="panel"><h2>System apps</h2>{apps.length ? <fieldset disabled={busy} aria-label="System apps">{apps.map(app => <div key={app.id}><label className="checkbox"><input type="checkbox" checked={app.selected} disabled={app.required} onChange={() => toggle(app.id)}/>{app.label}{app.required && <span className="muted"> · Required</span>}</label>{app.profiles.length > 1 && <select className="app-profile" aria-label={`${app.label} profile`} value={app.profile} disabled={!app.selected} onChange={event => chooseProfile(app.id, event.target.value)}>{app.profiles.map(profile => <option key={profile.id} value={profile.id}>{profile.label}</option>)}</select>}</div>)}</fieldset> : <p className="unavailable">None configured</p>}</section>;
}
