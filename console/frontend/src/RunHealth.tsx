import type { Snapshot } from './api';
import { Freshness, Health, Kind, type FaultView } from './streams';
import { StatusLabel } from './ui/StatusLabel';

type Runtime = Snapshot['composeRuntime'];
type Container = NonNullable<Runtime>['containers'][number];
const hasExit = (container: Container) => container.exitCode != null;
const processLabel = (container: Container) => hasExit(container) ? `Exited · ${container.exitCode}` : container.running === true ? 'Running' : 'Unknown';
const healthLabel = (health: number) => ({[Health.HEALTH_HEALTHY]:'Healthy', [Health.HEALTH_ACTIVE]:'Active', [Health.HEALTH_RECOVERING]:'Recovering'}[health] ?? 'Unknown');
const freshnessLabel = (freshness: number) => ({[Freshness.FRESHNESS_FRESH]:'Fresh', [Freshness.FRESHNESS_STALE]:'Stale'}[freshness] ?? 'Unknown');
const healthTone = (health: number) => health === Health.HEALTH_ACTIVE ? 'failure' as const : health === Health.HEALTH_RECOVERING ? 'attention' as const : health === Health.HEALTH_HEALTHY ? 'success' as const : 'neutral' as const;

/** Docker lifecycle facts retain unknown and exited states independently of reported health. */
export function ProcessPanel({ runtime, disconnected = false, historical = false }: { runtime?: Runtime; disconnected?: boolean; historical?: boolean }) {
  return <section className="panel"><h2>Service status</h2>
    {disconnected && !historical && <StatusLabel label="Connection lost" tone="attention"/>}
    {runtime?.error && <p className="notice">Observation incomplete: {runtime.error}</p>}
    {runtime?.containers?.length ? <ul className="service-status">{runtime.containers.map(container => <li key={container.containerId}>
      <div className="panel-heading"><strong className="mono break">{container.serviceName}</strong><StatusLabel label={processLabel(container)} tone={hasExit(container) && container.exitCode !== 0 ? 'failure' : 'neutral'}/></div>
      <span className="mono break muted">{container.containerId}</span>
    </li>)}</ul> : <p className="muted">{historical ? 'Service status unavailable for this saved run' : 'No Docker state observed'}</p>}
  </section>;
}

/** Watchdog app attribution, source identities and conditions at the bridge receipt time. */
export function FaultPanel({ faults, runtime, streams = [], disconnected = false, historical = false }: { faults?: FaultView; runtime?: Runtime; streams?: Snapshot['liveStreams']; disconnected?: boolean; historical?: boolean }) {
  const owners = new Set(streams.filter(stream => stream.kind === Kind.FAULT_PUBLICATION).map(stream => stream.appId));
  const exited = runtime?.containers.filter(container => hasExit(container) && (owners.has(container.serviceName) || container.serviceName === 'live-bridge')) ?? [];
  const active = faults?.conditions.filter(condition => condition.health === Health.HEALTH_ACTIVE).length ?? 0;
  const recovering = faults?.conditions.filter(condition => condition.health === Health.HEALTH_RECOVERING).length ?? 0;
  return <section className="panel reported-health"><h2>Reported health</h2>
    {!faults ? <p className="muted">No watchdog stream selected</p> : <>
      {disconnected && !historical && <StatusLabel label="Connection lost" tone="attention"/>}
      {exited.map(container => <p key={container.containerId}><StatusLabel label={`${container.serviceName} exited · ${container.exitCode}`} tone="attention"/></p>)}
      {faults.stale && <p><StatusLabel label="Waiting for complete snapshot" tone="attention"/></p>}
      {faults.receivedAtMs != null && <p className="muted">Last report received <time dateTime={new Date(faults.receivedAtMs).toISOString()}>{new Date(faults.receivedAtMs).toLocaleTimeString()}</time></p>}
      {(!faults.stale || faults.sources > 0) && faults.receivedAtMs == null && <p className="muted">Last report time unavailable</p>}
      {(!faults.stale || faults.sources > 0) && <p>{faults.connected}/{faults.sources} sources connected · {active} active · {recovering} recovering</p>}
      {faults.reportedSources.map(source => <section className="health-source" key={source.id}>
        <div className="panel-heading"><div><strong className="break">{source.app || 'Unknown app'}</strong><div className="mono break muted">Source {source.id}</div></div><StatusLabel label={source.connected === undefined ? 'Unknown' : source.connected ? 'Connected' : 'Disconnected'} tone={source.connected === false ? 'attention' : 'neutral'}/></div>
        {source.conditions.length ? <ul className="reported-conditions">{source.conditions.map(condition => <li key={condition.key?.condition ?? 0}>
          <div className="condition-name"><strong>{condition.definition?.component || `Condition ${condition.key?.condition ?? 0}`}</strong>{condition.definition?.resource && <span>{condition.definition.resource}</span>}</div>
          <span className="mono break">{condition.definition?.code}</span>
          <div className="condition-status"><div><span className="muted">Health</span><StatusLabel label={healthLabel(condition.health ?? 0)} tone={healthTone(condition.health ?? 0)}/></div><div><span className="muted">Freshness</span><StatusLabel label={freshnessLabel(condition.freshness ?? 0)} tone={condition.freshness === Freshness.FRESHNESS_STALE ? 'attention' : 'neutral'}/></div></div>
          {condition.latest?.detail && <p className="break">{condition.latest.detail}</p>}
        </li>)}</ul> : <p className="muted">No reported conditions</p>}
      </section>)}
      {!faults.reportedSources.length && !faults.stale && <p className="muted">No reported sources</p>}
    </>}
  </section>;
}
