import { useEffect, useRef, useState } from 'react';
import { api, ApiError, type Review, type Snapshot } from './api';
import { RunActivity } from './RunActivity';
import { Result } from './Result';
import { RunMetrics } from './RunMetrics';
import { SystemApps } from './SystemApps';
import { acceptsSnapshot, canStopRun, createRequestKey, requestKey, toggleProfile as nextProfiles } from './model';
import { ActionButton } from './ui/ActionButton';
import { PageFrame, type PagePhase } from './ui/PageFrame';
import { StatusLabel } from './ui/StatusLabel';

const active = new Set(['STARTING', 'RUNNING', 'STOPPING']);
const message = (error: unknown) => error instanceof ApiError ? ({ InvalidInput: 'Check the project directory and selected profiles.', StaleReview: 'The project changed after review. Review it again.', Conflict: 'Another run or request is already active.', Busy: 'The Console is busy. Try again.' }[error.code] ?? 'The request could not be completed.') : 'Connection lost; run may still be active.';

export function App() {
  const [phase, setPhase] = useState<PagePhase>('Project');
  const [directory, setDirectory] = useState('');
  const [profiles, setProfiles] = useState<string[]>([]);
  const [systemApps, setSystemApps] = useState<string[] | undefined>(undefined);
  const [available, setAvailable] = useState<string[]>([]);
  const [review, setReview] = useState<Review | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [stopPending, setStopPending] = useState(false);
  const [retry, setRetry] = useState(false);
  const [disconnected, setDisconnected] = useState(false);
  const [updated, setUpdated] = useState<number | null>(null);
  const [clock, setClock] = useState(Date.now());
  const generation = useRef(0);
  const selecting = useRef(false);
  const viewingRun = useRef(false);
  const key = useRef('');
  const expected = useRef('');

  useEffect(() => {
    let cancelled = false; let timer: ReturnType<typeof setTimeout>;
    const controller = new AbortController();
    const connect = async () => {
      try {
        await api.live(current => {
          if (!cancelled && acceptsSnapshot(expected.current, current)) {
            setSnapshot(current); setUpdated(Date.now()); setDisconnected(false);
            if (active.has(current.phase)) { viewingRun.current = false; expected.current = current.runId; selecting.current = false; setPhase('Run'); }
            else if (['FINISHED', 'BLOCKED'].includes(current.phase) && !selecting.current && !viewingRun.current) { setPhase('Result'); setStopPending(false); }
          }
        }, controller.signal);
      } catch { if (!cancelled) setDisconnected(true); }
      if (!cancelled) timer = setTimeout(connect, 1000);
    };
    void connect();
    return () => { cancelled = true; controller.abort(); clearTimeout(timer); };
  }, []);

  useEffect(() => {
    if (!disconnected) return;
    const timer = setInterval(() => setClock(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [disconnected]);

  const changeDirectory = (value: string) => { generation.current++; selecting.current = true; setDirectory(value); setReview(null); setAvailable([]); setProfiles([]); setSystemApps(undefined); setError(''); };
  const toggleProfile = (name: string) => { generation.current++; selecting.current = true; setProfiles(items => nextProfiles(items, name)); setReview(null); setError(''); };
  const prepare = async (appIds = systemApps) => {
    setBusy(true); setError(''); selecting.current = true;
    try { const next = await api.prepare(directory, profiles, appIds); key.current = ''; setSystemApps(next.systemApps.filter(app => app.selected && !app.required).map(app => app.id)); const discoveredProfiles = available.length === 0 && next.availableProfiles.length > 0; setAvailable(next.availableProfiles); if (!next.preparedId) { setReview(null); setError(next.warnings[0] ?? 'Select a profile to enable a service.'); setPhase('Project'); } else if (discoveredProfiles) { setReview(null); setPhase('Project'); } else { setReview(next); setPhase('Review'); } }
    catch (failure) { setError(message(failure)); }
    finally { setBusy(false); }
  };
  const toggleSystemApp = (id: string) => {
    if (!review || busy || review.systemApps.find(app => app.id === id)?.required) return;
    const apps = review.systemApps.map(app => app.id === id ? { ...app, selected: !app.selected } : app);
    const ids = apps.filter(app => app.selected && !app.required).map(app => app.id);
    setSystemApps(ids); setReview({ ...review, preparedId: '', systemApps: apps });
    void prepare(ids);
  };
  const start = async () => {
    if (!review?.preparedId || busy) return;
    key.current = requestKey(key.current, createRequestKey);
    generation.current++; expected.current = review.runId; selecting.current = false; setPhase('Run'); setRetry(false); setStopPending(false); setError('');
    setSnapshot({ steps: [], sequence: '0', lostEvents: '0', truncatedSteps: '0', overflowSteps: '0', progressIncomplete: false, outcome: null, cpuAvailable: false, reportAvailable: false, monitorReady: false, phase: 'STARTING', runId: review.runId, recordings: review.recordings, systemApps: { ids: review.systemApps.filter(app => app.selected && !app.required).map(app => app.id) } });
    try { await api.start(review.preparedId, key.current); }
    catch (failure) {
      if (failure instanceof ApiError && failure.code === 'Stopped') return;
      setError(message(failure));
      if (failure instanceof ApiError && failure.code === 'StaleReview') { setReview(null); selecting.current = true; setPhase('Project'); key.current = ''; }
      else if (!(failure instanceof ApiError)) setRetry(true);
    }
  };
  const stop = async () => {
    const id = snapshot?.runId || expected.current;
    if (!id || !canStopRun(snapshot, stopPending)) return;
    setStopPending(true); setRetry(false); setError('');
    try { await api.stop(id); }
    catch (failure) { setError(message(failure)); setStopPending(false); }
  };
  const newRun = () => { viewingRun.current = false; generation.current++; selecting.current = true; expected.current = ''; key.current = ''; setReview(null); setDirectory(''); setProfiles([]); setAvailable([]); setSystemApps(undefined); setStopPending(false); setRetry(false); setError(''); setPhase('Project'); };
  const state = snapshot?.phase ?? 'STARTING';
  const recordingEnabled = (snapshot?.recordings?.length ?? 0) > 0;
  const historical = ['FINISHED', 'BLOCKED'].includes(state);
  const navigateRun = (target: PagePhase) => { viewingRun.current = target === 'Run'; selecting.current = false; setPhase(target); };
  return <PageFrame phase={phase} onNavigate={historical && (phase === 'Run' || phase === 'Result') ? navigateRun : undefined}>
    {error && <div className="notice failure-notice" role="alert">{error}{retry && canStopRun(snapshot, stopPending) && <ActionButton variant="secondary" onClick={() => void start()}>Start run</ActionButton>}</div>}
    {disconnected && <div className="notice"><span role="status">Connection lost; run may still be active.</span> <span aria-live="off">{updated && `Last update ${Math.max(0, Math.floor((clock - updated) / 1000))} s ago.`}</span></div>}
    <div className="sr-only" aria-live="polite">{phase} {state === 'STOPPING' ? 'Stopping run' : ''}</div>
    {phase === 'Project' && <><div className="heading"><div><h1>Choose a project</h1></div></div><div className="columns"><section className="panel"><h2>Project directory</h2><p className="muted">Enter a directory beneath the approved project root.</p><label htmlFor="directory">Directory on this host</label><input id="directory" value={directory} onChange={event => changeDirectory(event.target.value)} placeholder="/projects/my-project" autoComplete="off"/>{available.length > 0 && <fieldset><legend>Optional profiles</legend>{available.map(profile => <label className="checkbox" key={profile}><input type="checkbox" checked={profiles.includes(profile)} onChange={() => toggleProfile(profile)}/>{profile}</label>)}</fieldset>}<div className="actions"><ActionButton disabled={!directory.trim() || busy} onClick={() => void prepare()}>{busy ? 'Checking…' : available.length ? 'Review project' : 'Continue'}</ActionButton></div></section><aside><section className="panel"><h2>Before you start</h2><p>Default Compose files and prebuilt local images required.</p></section></aside></div></>}
    {phase === 'Review' && review && <><div className="heading"><div><h1>Review project</h1></div><ActionButton onClick={() => void start()} disabled={busy || !review.preparedId}>Start run</ActionButton></div><div className="columns"><section className="panel"><div className="panel-heading"><h2>Application</h2><StatusLabel label={busy ? 'Checking…' : review.preparedId ? 'Ready to start' : 'Review required'} tone={review.preparedId && !busy ? 'success' : 'neutral'}/></div><p className="mono break">{directory}</p><h3>Enabled services</h3><table><thead><tr><th>Service</th><th>Local image</th></tr></thead><tbody>{review.services.map(service => <tr key={service.name}><td>{service.name}</td><td className="mono break">{service.imageId}</td></tr>)}</tbody></table>{review.warnings.map(warning => <p className="notice" key={warning}>{warning}</p>)}<div className="actions"><ActionButton variant="secondary" disabled={busy} onClick={() => { selecting.current = true; setPhase('Project'); }}>Change project</ActionButton>{!busy && !review.preparedId && <ActionButton variant="secondary" onClick={() => void prepare()}>Retry review</ActionButton>}</div></section><aside><SystemApps apps={review.systemApps} busy={busy} toggle={toggleSystemApp}/><section className="panel"><h2>Run settings</h2><p>Profiles</p><p>{profiles.length ? profiles.join(', ') : 'Default'}</p></section></aside></div></>}
    {phase === 'Run' && <><div className="heading"><div><h1>{historical ? 'Run activity' : state === 'STOPPING' || stopPending ? 'Stopping run' : 'Run in progress'}</h1><p>{(snapshot?.projectDirectory || directory) && <>{snapshot?.projectDirectory || directory} · </>}{(snapshot?.profiles?.length ?? 0) > 0 && <>{snapshot?.profiles?.join(', ')} · </>}Run <span className="mono">{snapshot?.runId || expected.current}</span></p></div>{historical ? <ActionButton variant="secondary" onClick={() => navigateRun('Result')}>View result</ActionButton> : <ActionButton variant="stop" disabled={!canStopRun(snapshot, stopPending)} onClick={() => void stop()}>{state === 'STOPPING' || stopPending ? 'Stopping…' : 'Stop run'}</ActionButton>}</div><div className="columns"><div><RunActivity snapshot={snapshot} stopping={state === 'STOPPING' || stopPending} disconnected={disconnected} historical={historical}/></div><aside><section className="panel"><h2>Recording</h2><StatusLabel label={!recordingEnabled ? 'Disabled' : historical ? (snapshot?.outcome?.recordingFinalized ? 'Saved' : 'Unavailable') : state === 'RUNNING' ? 'In progress' : 'Waiting for readiness'} tone={recordingEnabled && state === 'RUNNING' ? 'active' : 'neutral'}/></section><RunMetrics samples={snapshot?.metrics} cpuSnapshots={snapshot?.cpuSnapshots} cpuGroupNames={snapshot?.cpuGroupNames} historical={historical} disconnected={disconnected} incomplete={snapshot?.metricsIncomplete}/></aside></div></>}
    {phase === 'Result' && snapshot && <Result snapshot={snapshot} newRun={newRun} viewRun={() => navigateRun('Run')}/>}
  </PageFrame>;
}
