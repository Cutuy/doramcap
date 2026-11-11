import { Activity, Cpu, Database, Radio, SlidersHorizontal, MemoryStick, ShieldAlert, Layers, Box, Cog, Workflow } from 'lucide-react';
import data from './.generated/overview.json';
import ConsoleInstall from './ConsoleInstall';
import { componentHref, displayName, routeHref, type Catalog, type Route } from './model';
import type { OverviewData, Mapping } from './overview-model';
const overview = data as OverviewData;
const icons: Record<string, typeof Activity> = { Resources: Cpu, Memory: MemoryStick, Observability: Activity, Storage: Database, Faults: ShieldAlert, Configuration: SlidersHorizontal, Telemetry: Radio, Runtime: Workflow };
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
    <ConsoleInstall repository={catalog.repository} />
    <section className="scenario-picker" aria-labelledby="scenario-heading"><h2 id="scenario-heading">Explore use cases</h2><nav className="scenario-tabs" aria-label="Use cases">{overview.useCases.map(u => <a key={u.id} href={link({ useCase: u.id })} aria-current={useCase?.id === u.id ? 'page' : undefined}>{u.title}</a>)}</nav></section>
    {invalid && <p role="alert">This selection is not in the current overview. <a href="#/overview">Show the first use case</a>.</p>}
    {selected ? <section className="mapping-detail" aria-live="polite"><div className="mapping-heading">{statusLabels[selected.status] && <div className="eyebrow">{statusLabels[selected.status]}</div>}<h2>{selected.title}</h2><p>{selected.summary}</p>{integration && <a href={integration.url} target="_blank" rel="noreferrer">Upstream project ↗</a>}</div><div className="mapping-components">{components(selected, 'apps')}{components(selected, 'crates')}</div></section> : null}
    <section className="platform-stack" aria-labelledby="architecture-heading">
      <h2 id="architecture-heading" className="stack-heading eyebrow">System architecture</h2>

      <a
        className={highlights.includes('orchestration') ? 'orchestration-layer highlighted' : 'orchestration-layer'}
        href={link({ ...context, capability: 'orchestration' })}
        aria-current={capability?.id === 'orchestration' && !integration ? 'true' : undefined}
      >
        <Workflow size={16} aria-hidden="true" />Console
      </a>

      <div className="layer-connector" aria-hidden="true" />
      <div className="stack-services">
        <div>
          <section className="stack-apps" aria-label="Applications">
            <div className="stack-block"><Layers size={16} aria-hidden="true" />Your apps</div>
            <div className="stack-block"><Cog size={16} aria-hidden="true" />System apps</div>
          </section>

          <div className="layer-connector" aria-hidden="true" />
          <section className="runtime-layer" aria-labelledby="runtime-heading">
            <div className="layer-heading">
              <h3 id="runtime-heading">doramcap runtime</h3>
              <span>Optional to adopt</span>
            </div>
            <div className="capability-grid">
              {overview.capabilities.map(c => {
                const Icon = icons[c.category] ?? Layers;
                return <a
                  key={c.id}
                  href={link({ ...context, capability: c.id })}
                  className={highlights.includes(c.id) ? 'capability highlighted' : 'capability'}
                  aria-current={capability?.id === c.id && !integration ? 'true' : undefined}
                >
                  <Icon size={14} strokeWidth={1.6} aria-hidden="true" />{c.title}
                </a>;
              })}
            </div>
          </section>

          <div className="layer-connector" aria-hidden="true" />
          <section className="lifecycle-layer" aria-labelledby="lifecycle-heading">
            <div className="layer-heading"><h3 id="lifecycle-heading">Lifecycle</h3></div>
            <div className="lifecycle-grid">
              <div className="stack-block"><Box size={16} aria-hidden="true" />Docker containers</div>
              <a className="stack-block" href={link({ ...context, integration: 'holoscan' })} aria-current={integration?.id === 'holoscan' ? 'true' : undefined}>
                <Workflow size={16} aria-hidden="true" />Holoscan containers
              </a>
              <div className="stack-block"><Cog size={16} aria-hidden="true" />systemd units</div>
            </div>
          </section>
        </div>
        <section className="history-layer" aria-labelledby="history-heading">
          <div className="layer-heading"><h3 id="history-heading">Run history</h3></div>
          <p>Past runs, recordings, and comparisons.</p>
        </section>
      </div>

      <div className="layer-connector" aria-hidden="true" />
      <section className="software-layer" aria-labelledby="software-heading">
        <div className="layer-heading"><h3 id="software-heading">Platform</h3></div>
        {(['communication', 'execution'] as const).map(group => <div className="software-group" key={group}>
          <h4>{group === 'execution' ? 'Runtime' : 'Data'}</h4>
          <div className="software-links">
            {overview.integrations.filter(i => (i.group ?? 'communication') === group).map(i => <a
              key={i.id}
              href={link({ ...context, integration: i.id })}
              aria-current={integration?.id === i.id ? 'true' : undefined}
              className={useCase?.integrations.includes(i.id) || capability?.integrations.includes(i.id) ? 'highlighted' : ''}
            >
              {i.title}
              {i.status === 'optional' && <small>optional</small>}
              {i.status === 'planned' && <small>planned</small>}
              {i.status === 'platform-dependent' && <small>platform-dependent</small>}
            </a>)}
          </div>
        </div>)}
      </section>

      <div className="layer-connector" aria-hidden="true" />
      <section className="platform-layer">
        <div className="layer-heading"><h3>Compute</h3></div>
        <p>{overview.platforms.join(' · ')}</p>
      </section>
      <div className="layer-connector" aria-hidden="true" />
      <section className="platform-layer">
        <div className="layer-heading"><h3>Embodiment</h3></div>
        <p>{overview.embodiments.join(' · ')}</p>
      </section>
    </section>
  </div>;
}
