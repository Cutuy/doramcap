import { Phase } from './api';
import { useEffect, useRef, useState } from 'react';
import { wire, api, ApiError, PREPARING_RUN, type HistoryEntry, type Review, type Snapshot } from './api';
import { RunActivity } from './RunActivity';
import { Result } from './Result';
import { RunMetrics } from './RunMetrics';
import { ReviewPreview } from './Comparison';
import { HistoryList, HistoryRunPage } from './History';
import { LiveModel, type LiveView } from './streams';
import { SystemApps } from './SystemApps';
import { acceptsSnapshot, canStopRun, createRequestKey, historyRoute, requestKey, runAgainBlocked, selectedProfiles, toggleProfile as nextProfiles } from './model';
import { ActionButton } from './ui/ActionButton';
import { PageFrame, type PagePhase } from './ui/PageFrame';
import { StatusLabel } from './ui/StatusLabel';

const active = new Set([Phase.STARTING, Phase.RUNNING, Phase.STOPPING]);
const messages: Partial<Record<ApiError['code'], string>> = { InvalidInput: 'The project could not be reviewed. Check its Compose files and the selected profiles.', StaleReview: 'The project changed after review. Review it again.', Conflict: 'Another run or request is already active.', Busy: 'The Console is busy. Try again.', ImageDownload: 'An image could not be downloaded. Check network and registry access, then review again. Details: sudo journalctl -u "doramcap-console-*"', Stopped: 'Review cancelled.' };
const message = (error: unknown) => error instanceof ApiError ? [messages[error.code] ?? 'The request could not be completed.', error.detail].filter(Boolean).join('\n') : 'Connection lost; run may still be active.';

export function App() {
  const [phase, setPhase] = useState<PagePhase>('Project');
  const [route, setRoute] = useState(() => typeof location === 'undefined' ? null : historyRoute(location.hash));
  const [profiles, setProfiles] = useState<string[]>([]);
  const [systemApps, setSystemApps] = useState<string[] | undefined>(undefined);
  const [appProfiles, setAppProfiles] = useState<Record<string, string>>({});
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
  const baselineRun = useRef('');
  const liveModel = useRef(new LiveModel());
  const [liveView, setLiveView] = useState<LiveView | undefined>(undefined);

  useEffect(() => {
    let cancelled = false; let timer: ReturnType<typeof setTimeout>;
    const controller = new AbortController();
    const connect = async () => {
      try {
        await api.live(current => {
          if (!cancelled && acceptsSnapshot(expected.current, current)) {
            liveModel.current.configure(current.runId, current.liveStreams); setLiveView(liveModel.current.view());
            setSnapshot(current); setUpdated(Date.now()); setDisconnected(false);
            if (active.has(current.phase)) { viewingRun.current = false; expected.current = current.runId; selecting.current = false; setPhase('Run'); }
            else if ([Phase.FINISHED, Phase.BLOCKED].includes(current.phase) && !selecting.current && !viewingRun.current) { setPhase('Result'); setStopPending(false); }
          }
        }, controller.signal, frame => { if (!cancelled && liveModel.current.receive(frame)) setLiveView(liveModel.current.view()); });
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

  useEffect(() => {
    const follow = () => setRoute(historyRoute(location.hash));
    window.addEventListener('hashchange', follow);
    return () => window.removeEventListener('hashchange', follow);
  }, []);

  const toggleProfile = (name: string) => { generation.current++; selecting.current = true; setProfiles(items => nextProfiles(items, name)); setReview(null); setError(''); };
  const prepare = async (appIds = systemApps, rerun?: { profiles: string[] }, launchProfiles = appProfiles) => {
    const revision = ++generation.current;
    setBusy(true); setError(''); selecting.current = true;
    try {
      const next = await api.prepare(rerun?.profiles ?? profiles, appIds, launchProfiles, baselineRun.current);
      if (revision !== generation.current) return;
      key.current = '';
      setSystemApps(next.systemApps.filter(app => app.selected && !app.required).map(app => app.id));
      setAppProfiles(selectedProfiles(next.systemApps));
      const discoveredProfiles = !rerun && available.length === 0 && next.availableProfiles.length > 0;
      setAvailable(next.availableProfiles);
      if (!next.preparedId) { setReview(null); setError(next.warnings[0] ?? 'Select a profile to enable a service.'); setPhase('Project'); }
      else if (discoveredProfiles) { setReview(null); setPhase('Project'); }
      else { setReview(next); setPhase('Review'); }
    }
    catch (failure) {
      if (revision !== generation.current) return;
      // A selection the project or host no longer offers must not block a fresh review.
      if (rerun && failure instanceof ApiError && failure.code === 'InvalidInput') { setProfiles([]); setSystemApps(undefined); setAppProfiles({}); setError('The previous selection is no longer valid. Continue to choose again.'); }
      else setError(message(failure));
    }
    finally { setBusy(false); }
  };
  const toggleSystemApp = (id: string) => {
    if (!review || busy || review.systemApps.find(app => app.id === id)?.required) return;
    const apps = review.systemApps.map(app => app.id === id ? wire.SystemApp.create({ ...app, selected: !app.selected }) : app);
    const ids = apps.filter(app => app.selected && !app.required).map(app => app.id);
    const chosen = selectedProfiles(apps);
    setSystemApps(ids); setAppProfiles(chosen); setReview(wire.Review.create({ ...review, preparedId: '', systemApps: apps }));
    void prepare(ids, undefined, chosen);
  };
  const chooseLaunchProfile = (id: string, profile: string) => {
    if (!review || busy) return;
    const apps = review.systemApps.map(app => app.id === id ? wire.SystemApp.create({ ...app, profile }) : app);
    const chosen = selectedProfiles(apps);
    setAppProfiles(chosen); setReview(wire.Review.create({ ...review, preparedId: '', systemApps: apps }));
    void prepare(systemApps, undefined, chosen);
  };
  const start = async () => {
    if (!review?.preparedId || busy) return;
    key.current = requestKey(key.current, createRequestKey);
    generation.current++; expected.current = review.runId; selecting.current = false; setPhase('Run'); setRetry(false); setStopPending(false); setError('');
    liveModel.current.configure(review.runId, review.liveStreams); setLiveView(liveModel.current.view());
    setSnapshot(wire.Status.create({
      phase: Phase.STARTING, runId: review.runId, recordings: review.recordings, liveStreams: review.liveStreams,
      systemApps: wire.SystemAppSelection.create({ ids: review.systemApps.filter(app => app.selected && !app.required).map(app => app.id), profiles: selectedProfiles(review.systemApps) }),
    }));
    try { await api.start(review.preparedId, key.current); }
    catch (failure) {
      if (failure instanceof ApiError && failure.code === 'Stopped') return;
      setError(message(failure));
      if (failure instanceof ApiError) { setReview(null); selecting.current = true; expected.current = ''; setPhase('Project'); key.current = ''; }
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
  const newRun = () => { baselineRun.current = ''; viewingRun.current = false; generation.current++; selecting.current = true; expected.current = ''; key.current = ''; setReview(null); setProfiles([]); setAvailable([]); setSystemApps(undefined); setAppProfiles({}); setStopPending(false); setRetry(false); setError(''); setPhase('Project'); };
  /** Prefill a past run's selection, then enter the normal Review flow. */
  const runAgain = (entry: HistoryEntry) => {
    if (runAgainBlocked(snapshot, busy)) return;
    newRun();
    baselineRun.current = entry.runId;
    const ids = entry.systemApps?.ids ?? undefined, rerun = { profiles: entry.profiles };
    const launchProfiles = entry.systemApps?.profiles ?? {};
    setProfiles(rerun.profiles); setSystemApps(ids); setAppProfiles(launchProfiles);
    location.hash = '#/';
    void prepare(ids, rerun, launchProfiles);
  };
  const state = snapshot?.phase ?? Phase.STARTING;
  const recordingEnabled = (snapshot?.recordings?.length ?? 0) > 0;
  const historical = [Phase.FINISHED, Phase.BLOCKED].includes(state);
  const navigateRun = (target: PagePhase) => { viewingRun.current = target === 'Run'; selecting.current = false; setPhase(target); };
  const refresh = `${snapshot?.runId}:${snapshot?.phase}`;
  return <PageFrame phase={phase} history={!!route} onNavigate={historical && (phase === 'Run' || phase === 'Result') ? navigateRun : undefined}>
    {route ? <>{active.has(state) && !!snapshot?.runId && <p className="notice" role="status">A run is in progress. <a href="#/">View live run</a></p>}{route.runId ? <HistoryRunPage id={route.runId} refresh={refresh} runAgain={runAgain} runActive={runAgainBlocked(snapshot, busy)}/> : <HistoryList refresh={refresh} selected={route.compare}/>}</> : <>
    {error && <div className="notice failure-notice" role="alert">{error}{retry && canStopRun(snapshot, stopPending) && <ActionButton variant="secondary" onClick={() => void start()}>Start run</ActionButton>}</div>}
    {disconnected && <div className="notice"><span role="status">Connection lost; run may still be active.</span> <span aria-live="off">{updated && `Last update ${Math.max(0, Math.floor((clock - updated) / 1000))} s ago.`}</span></div>}
    <div className="sr-only" aria-live="polite">{phase} {state === Phase.STOPPING ? 'Stopping run' : ''}</div>
    {phase === 'Project' && <><div className="heading"><div><h1>Start a run</h1></div></div><div className="columns"><section className="panel"><h2>Project</h2><p className="mono break">{snapshot?.projectDirectory}</p>{available.length > 0 && <fieldset><legend>Optional profiles</legend>{available.map(profile => <label className="checkbox" key={profile}><input type="checkbox" checked={profiles.includes(profile)} onChange={() => toggleProfile(profile)}/>{profile}</label>)}</fieldset>}<div className="actions"><ActionButton disabled={busy} onClick={() => void prepare()}>{busy ? 'Checking…' : available.length ? 'Review project' : 'Continue'}</ActionButton>{busy && <ActionButton variant="secondary" onClick={() => void api.stop(PREPARING_RUN).catch(() => {})}>Cancel</ActionButton>}</div></section><aside><section className="panel"><h2>Before you start</h2><p>This Console runs one Compose project. Release installations download missing images while preparing the review.</p></section></aside></div></>}
    {phase === 'Review' && review && <><div className="heading"><div><h1>Review project</h1></div><ActionButton onClick={() => void start()} disabled={busy || !review.preparedId}>Start run</ActionButton></div><div className="columns"><section className="panel"><div className="panel-heading"><h2>Application</h2><StatusLabel label={busy ? 'Checking…' : review.preparedId ? 'Ready to start' : 'Review required'} tone={review.preparedId && !busy ? 'success' : 'neutral'}/></div><p className="mono break">{snapshot?.projectDirectory}</p><h3>Enabled services</h3><table><thead><tr><th>Service</th><th>Local image</th></tr></thead><tbody>{review.services.map(service => <tr key={service.name}><td>{service.name}</td><td className="mono break">{service.imageId}</td></tr>)}</tbody></table>{review.warnings.map(warning => <p className="notice" key={warning}>{warning}</p>)}<div className="actions"><ActionButton variant="secondary" disabled={busy} onClick={() => { selecting.current = true; setPhase('Project'); }}>Back</ActionButton>{!busy && !review.preparedId && <ActionButton variant="secondary" onClick={() => void prepare()}>Retry review</ActionButton>}</div></section><aside>{review.comparison && <ReviewPreview comparison={review.comparison}/>}<SystemApps apps={review.systemApps} busy={busy} toggle={toggleSystemApp} chooseProfile={chooseLaunchProfile}/><section className="panel"><h2>Run settings</h2><p>Profiles</p><p>{profiles.length ? profiles.join(', ') : 'Default'}</p></section></aside></div></>}
    {phase === 'Run' && <><div className="heading"><div><h1>{historical ? 'Run activity' : state === Phase.STOPPING || stopPending ? 'Stopping run' : 'Run in progress'}</h1><p>{snapshot?.projectDirectory && <>{snapshot.projectDirectory} · </>}{(snapshot?.profiles?.length ?? 0) > 0 && <>{snapshot?.profiles?.join(', ')} · </>}Run <span className="mono">{snapshot?.runId || expected.current}</span></p></div>{historical ? <ActionButton variant="secondary" onClick={() => navigateRun('Result')}>View result</ActionButton> : <ActionButton variant="stop" disabled={!canStopRun(snapshot, stopPending)} onClick={() => void stop()}>{state === Phase.STOPPING || stopPending ? 'Stopping…' : 'Stop run'}</ActionButton>}</div><div className="columns"><div><RunActivity snapshot={snapshot} stopping={state === Phase.STOPPING || stopPending} disconnected={disconnected} historical={historical}/></div><aside><section className="panel"><h2>Recording</h2><StatusLabel label={!recordingEnabled ? 'Disabled' : historical ? (snapshot?.outcome?.recordingFinalized ? 'Saved' : 'Unavailable') : state === Phase.RUNNING ? 'In progress' : 'Waiting for readiness'} tone={recordingEnabled && state === Phase.RUNNING ? 'active' : 'neutral'}/></section><RunMetrics recordingEnabled={recordingEnabled} samples={snapshot?.metrics} live={liveView} historical={historical} disconnected={disconnected} incomplete={snapshot?.metricsIncomplete}/></aside></div></>}
    {phase === 'Result' && snapshot && <Result snapshot={snapshot} newRun={newRun} viewRun={() => navigateRun('Run')}/>}
    </>}
  </PageFrame>;
}
