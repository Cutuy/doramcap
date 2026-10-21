import { Activity, Cpu, Database, Radio, SlidersHorizontal, MemoryStick, ShieldAlert, Layers, ScanEye, Network, Waypoints, Workflow } from 'lucide-react';
import data from './.generated/overview.json';
import { componentHref, displayName, routeHref, type Catalog, type Route } from './model';
import type { OverviewData, Mapping } from './overview-model';
const overview = data as OverviewData;
const icons: Record<string, typeof Activity> = { Resources: Cpu, Memory: MemoryStick, Observability: Activity, Storage: Database, Faults: ShieldAlert, Configuration: SlidersHorizontal, Telemetry: Radio };
const workloadIcons: Record<string, typeof Activity> = { Perception: ScanEye, 'Learned policies': Network, 'Planning & control': Waypoints, 'Robotics workloads': Workflow };
const statusLabels: Record<string, string> = { 'building-blocks': '', planned: 'Planned', integrated: '', optional: 'Optional', 'platform-dependent': 'Platform-dependent', illustrative: '', validated: 'Validated use case' };
function link(values: Record<string, string>) { return routeHref('/overview', new URLSearchParams(values)); }
export default function Overview({ catalog, route }: { catalog: Catalog; route: Route }) {
  const useCaseId = route.params.get('useCase') ?? overview.useCases[0].id;
  const capabilityId = route.params.get('capability');
  const integrationId = route.params.get('integration');
  const useCase = overview.useCases.find(u => u.id === useCaseId);
  const capability = overview.capabilities.find(c => c.id === capabilityId);
  const integration = overview.integrations.find(i => i.id === integrationId);
  const selected = integration ?? capability ?? useCase;
  const context: Record<string, string> = useCase ? { useCase: useCase.id } : {};
  const invalid = (useCaseId && !useCase) || (capabilityId && !capability) || (integrationId && !integration);
  const highlights = integration ? overview.capabilities.filter(c => c.integrations.includes(integration.id)).map(c => c.id) : useCase?.capabilities ?? [];
  function components(item: Mapping, kind: 'apps' | 'crates') {
    const entries = item.components.map(id => catalog.components.find(c => c.id === id)!).filter(c => c.kind === kind);
    return entries.length ? <section><h3>{kind === 'apps' ? 'Apps' : 'Crates'}</h3><div className="mapping-links">{entries.map(c => <a key={c.id} href={componentHref(c)}>{displayName(c.name)} <span aria-hidden="true">→</span></a>)}</div></section> : null;
  }
  return <div className="overview"><div className="intro"><h1>{overview.title}</h1></div>
    <section className="scenario-picker" aria-labelledby="scenario-heading"><h2 id="scenario-heading">Explore use cases</h2><nav className="scenario-tabs" aria-label="Use cases">{overview.useCases.map(u => <a key={u.id} href={link({ useCase: u.id })} aria-current={useCase?.id === u.id ? 'page' : undefined}>{u.title}</a>)}</nav></section>
    {invalid && <p role="alert">This selection is not in the current overview. <a href="#/overview">Show the first use case</a>.</p>}
    {selected ? <section className="mapping-detail" aria-live="polite"><div className="mapping-heading">{statusLabels[selected.status] && <div className="eyebrow">{statusLabels[selected.status]}</div>}<h2>{selected.title}</h2><p>{selected.summary}</p>{integration && <a href={integration.url} target="_blank" rel="noreferrer">Upstream project ↗</a>}</div><div className="mapping-components">{components(selected, 'apps')}{components(selected, 'crates')}</div></section> : null}
    <section className="platform-stack" aria-label="Application platform stack"><div className="stack-heading"><span className="eyebrow">Application platform</span></div><section className="workload-layer"><div className="layer-heading"><h2>Your applications</h2></div><div className="workload-labels">{overview.workloads.map(w => { const Icon = workloadIcons[w] ?? Layers; return <span key={w}><Icon size={16} strokeWidth={1.6} aria-hidden="true" />{w}</span>; })}</div></section>
    <div className="layer-connector" aria-hidden="true" />
    <div className="foundation-row"><section className="foundation-layer"><div className="layer-heading"><h2>The doramcap foundation</h2></div><div className="capability-grid">{overview.capabilities.map(c => { const Icon = icons[c.category] ?? Layers; return <a key={c.id} href={link({ ...context, capability: c.id })} className={`capability${highlights.includes(c.id) ? ' highlighted' : ''}`} aria-current={capability?.id === c.id && !integration ? 'true' : undefined}><span><Icon size={14} strokeWidth={1.6} aria-hidden="true" />{c.title}</span><small>{c.summary}</small></a>; })}</div></section><aside className="existing-middleware"><h2>Existing middleware</h2></aside></div>
    <div className="layer-connector" aria-hidden="true" /><section className="software-layer" aria-labelledby="software-heading"><div className="layer-heading"><h2 id="software-heading">Platform</h2></div>{(['communication', 'execution'] as const).map(group => <div className="software-group" key={group}><h3>{group === 'execution' ? 'Runtime' : 'Data'}</h3><div className="software-links">{overview.integrations.filter(i => (i.group ?? 'communication') === group).map(i => <a key={i.id} href={link({ ...context, integration: i.id })} aria-current={integration?.id === i.id ? 'true' : undefined} className={useCase?.integrations.includes(i.id) || capability?.integrations.includes(i.id) ? 'highlighted' : ''}>{i.title}{i.status === 'optional' && <small>optional</small>}{i.status === 'platform-dependent' && <small>platform-dependent</small>}</a>)}</div></div>)}</section><div className="layer-connector" aria-hidden="true" /><section className="platform-layer"><div className="layer-heading"><h2>Compute</h2></div><p>{overview.platforms.join(' · ')}</p></section><div className="layer-connector" aria-hidden="true" /><section className="platform-layer embodiments-layer"><div className="layer-heading"><h2>Embodiments</h2></div><p>{overview.embodiments.join(' · ')}</p></section></section>
  </div>;
}
