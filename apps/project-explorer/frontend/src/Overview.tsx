import { useEffect, useRef, type MouseEvent } from 'react';
import { Activity, Cpu, Database, Radio, SlidersHorizontal, MemoryStick, ShieldAlert, Layers, Box, Cog, Workflow, History, ScanLine, GitCompareArrows, ListTree, ClipboardCheck, Play, Download, X } from 'lucide-react';
import data from './.generated/overview.json';
import ConsoleInstall from './ConsoleInstall';
import { componentHref, displayName, routeHref, type Catalog, type Route } from './model';
import type { OverviewData, Mapping } from './overview-model';
const overview = data as OverviewData;
const icons: Record<string, typeof Activity> = { Resources: Cpu, Memory: MemoryStick, Observability: Activity, Storage: Database, Faults: ShieldAlert, Configuration: SlidersHorizontal, Telemetry: Radio, Runtime: Workflow };
const statusLabels: Record<string, string> = { 'building-blocks': '', planned: 'Planned', integrated: '', optional: 'Optional', 'platform-dependent': 'Platform-dependent', illustrative: '', validated: 'Validated use case' };
const deviceActions: { title: string; Icon: typeof Activity; descriptions: Partial<Record<string, string>> }[] = [
  { title: 'Review run setup', Icon: ClipboardCheck, descriptions: {
    repeat: 'Inspect the resolved Compose services.',
    perception: 'Review the pipeline nodes and inputs.',
  } },
  { title: 'Launch experiments', Icon: Play, descriptions: {
    repeat: 'Start your existing Compose project.',
    perception: 'Run the selected dataflow.',
  } },
  { title: 'Monitor live runs', Icon: Radio, descriptions: {
    perception: 'Watch activity as the pipeline processes inputs.',
    latency: 'Watch CPU activity as the workload runs.',
    fault: 'Watch for liveness changes and fault reports.',
  } },
  { title: 'Download recordings', Icon: Download, descriptions: {
    repeat: 'Save the run’s recordings for inspection.',
    perception: 'Keep pipeline evidence for later inspection.',
    latency: 'Keep timing traces for the investigation.',
    fault: 'Keep evidence from around the fault.',
  } },
  { title: 'Run history', Icon: History, descriptions: {
    repeat: 'Find a previous run and run it again.',
    fault: 'Open the run where the fault occurred.',
  } },
  { title: 'System snapshots', Icon: ScanLine, descriptions: {
    latency: 'Check the machine configuration captured with the run.',
  } },
  { title: 'A/B comparison', Icon: GitCompareArrows, descriptions: {
    repeat: 'Compare the setup and results of two runs.',
    latency: 'Compare a baseline run with a run after the change.',
  } },
  { title: 'Performance analysis', Icon: Activity, descriptions: {
    perception: 'Trace an output back to the inputs that produced it.',
    latency: 'Relate a missed deadline to activity timing and CPU usage.',
    fault: 'Inspect activity surrounding the fault.',
  } },
];

function link(values: Record<string, string>) { return routeHref('/overview', new URLSearchParams(values)); }
function showDetails(panel: HTMLElement | null, source?: HTMLAnchorElement) {
  if (!panel?.showPopover) return;
  if (panel.matches(':popover-open')) panel.hidePopover();
  panel.toggleAttribute('data-anchored', Boolean(source));
  panel.showPopover({ source });
  panel.querySelector('button')?.focus({ preventScroll: true });
}

export default function Overview({ catalog, route }: { catalog: Catalog; route: Route }) {
  const details = useRef<HTMLElement>(null);
  const clicked = useRef<HTMLAnchorElement | null>(null);
  const useCaseId = route.params.get('useCase') ?? overview.useCases[0].id;
  const capabilityId = route.params.get('capability');
  const integrationId = route.params.get('integration');
  const useCase = overview.useCases.find(u => u.id === useCaseId);
  const capability = overview.capabilities.find(c => c.id === capabilityId);
  const integration = overview.integrations.find(i => i.id === integrationId);
  const selected = integration ?? capability;
  const context: Record<string, string> = useCase ? { useCase: useCase.id } : {};
  const useCaseComponents = useCase?.components ?? [];
  const invalid = (useCaseId && !useCase) || (capabilityId && !capability) || (integrationId && !integration);
  const highlights = integration ? overview.capabilities.filter(c => c.integrations.includes(integration.id)).map(c => c.id) : useCase?.capabilities ?? [];
  useEffect(() => {
    const source = clicked.current ?? undefined;
    clicked.current = null;
    if (!invalid && selected) showDetails(details.current, source);
    else if (details.current?.hidePopover && details.current.matches(':popover-open')) details.current.hidePopover();
  }, [route, invalid, selected]);

  function openDetails(event: MouseEvent<HTMLAnchorElement>) {
    if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    const source = event.currentTarget;
    if (source.hash === window.location.hash) showDetails(details.current, source);
    else clicked.current = source;
  }

  const detailLink = { onClick: openDetails, 'aria-haspopup': 'dialog' as const, 'aria-controls': 'overview-details' };

  function components(ids: Mapping['components'], kind: 'apps' | 'crates', Heading: 'h3' | 'h5' = 'h3') {
    const entries = ids.map(id => catalog.components.find(c => c.id === id)!).filter(c => c.kind === kind);
    return entries.length ? <section>
      <Heading>{kind === 'apps' ? 'Apps' : 'Crates'}</Heading>
      <div className="mapping-links">{entries.map(c => <a
        key={c.id}
        className={useCaseComponents.includes(c.id) ? 'highlighted' : undefined}
        href={componentHref(c)}
      >{displayName(c.name)} <span aria-hidden="true">→</span></a>)}</div>
    </section> : null;
  }
  return <div className="overview"><div className="intro"><h1>{overview.title}</h1></div>
    <ConsoleInstall repository={catalog.repository} />
    <section className="scenario-picker" aria-labelledby="scenario-heading"><h2 id="scenario-heading">Explore use cases</h2><nav className="scenario-tabs" aria-label="Use cases">{overview.useCases.map(u => <a key={u.id} href={link({ useCase: u.id })} aria-current={useCase?.id === u.id ? 'page' : undefined}>{u.title}</a>)}</nav></section>
    {invalid && <p role="alert">This selection is not in the current overview. <a href={link({ useCase: overview.useCases[0].id })}>Show the first use case</a>.</p>}
    {selected && <section
      ref={details}
      id="overview-details"
      className="mapping-detail"
      popover="auto"
      role="dialog"
      aria-labelledby="overview-detail-title"
    >
      <div className="mapping-header">
        <div>
          {statusLabels[selected.status] && <div className="eyebrow">{statusLabels[selected.status]}</div>}
          <h2 id="overview-detail-title">{selected.title}</h2>
        </div>
        <button className="mapping-close" type="button" aria-label="Close details" onClick={() => details.current?.hidePopover()}>
          <X size={18} aria-hidden="true" />
        </button>
      </div>
      <p>{selected.summary}</p>
      {integration && <a className="mapping-upstream" href={integration.url} target="_blank" rel="noreferrer">Upstream project ↗</a>}
      <div className="mapping-components">{components(selected.components, 'apps')}{components(selected.components, 'crates')}</div>
    </section>}
    <section className="user-device-layer" aria-labelledby="user-device-heading">
      <h2 id="user-device-heading" className="stack-heading">Your device</h2>
      <ul className="user-device-grid">
        {deviceActions.map(({ title, Icon, descriptions }) => {
          const description = useCase ? descriptions[useCase.id] : undefined;
          return <li key={title} className={description ? 'stack-block highlighted' : 'stack-block'}>
            <Icon size={16} aria-hidden="true" />
            <span>{title}{description && <small className="tile-caption">{description}</small>}</span>
          </li>;
        })}
      </ul>
    </section>

    <section className="platform-stack" aria-labelledby="architecture-heading">
      <h2 id="architecture-heading" className="stack-heading">SoC or workstation</h2>

      <section className="stack-apps" aria-label="Applications">
        <div className="stack-block"><Layers size={16} aria-hidden="true" />Your apps</div>
        <a
          className={highlights.includes('orchestration') ? 'stack-block highlighted' : 'stack-block'}
          {...detailLink}
          href={link({ ...context, capability: 'orchestration' })}
          aria-current={capability?.id === 'orchestration' && !integration ? 'true' : undefined}
        >
          <Workflow size={16} aria-hidden="true" />Console
        </a>
        <div className="stack-block"><Cog size={16} aria-hidden="true" />System apps</div>
      </section>

      <div className="layer-connector" aria-hidden="true" />
      <section className="runtime-layer" aria-labelledby="runtime-heading">
        <div className="layer-heading">
          <h3 id="runtime-heading">doramcap runtime</h3>
          <span>optional</span>
        </div>
        <div className="capability-grid">
          {overview.capabilities.map(c => {
            const Icon = icons[c.category] ?? Layers;
            const highlighted = highlights.includes(c.id);
            const related = c.components.filter(id => useCaseComponents.includes(id));
            const titleId = `capability-title-${c.id}`;
            return <article key={c.id} className={highlighted ? 'capability highlighted' : 'capability'}>
              <a
                {...detailLink}
                href={link({ ...context, capability: c.id })}
                className="capability-detail"
                aria-labelledby={titleId}
                aria-current={capability?.id === c.id && !integration ? 'true' : undefined}
              >
                <h4 id={titleId} className="capability-title"><Icon size={14} strokeWidth={1.6} aria-hidden="true" />{c.title}</h4>
                {highlighted && <small className="tile-caption">{c.summary}</small>}
              </a>
              {highlighted && related.length > 0 && <div className="mapping-components">
                {components(related, 'apps', 'h5')}
                {components(related, 'crates', 'h5')}
              </div>}
            </article>;
          })}
        </div>
      </section>

      <div className="layer-connector" aria-hidden="true" />
      <section className="lifecycle-layer" aria-labelledby="lifecycle-heading">
        <div className="layer-heading"><h3 id="lifecycle-heading">Lifecycle</h3></div>
        <div className="lifecycle-grid">
          <div className="stack-block"><Box size={16} aria-hidden="true" />Docker containers</div>
          <a className="stack-block" {...detailLink} href={link({ ...context, integration: 'holoscan' })} aria-current={integration?.id === 'holoscan' ? 'true' : undefined}>
            <Workflow size={16} aria-hidden="true" />Holoscan containers
          </a>
          <div className="stack-block"><Cog size={16} aria-hidden="true" />systemd units</div>
          <div className="stack-block"><ListTree size={16} aria-hidden="true" />Supervisor units</div>
        </div>
      </section>

      <div className="layer-connector" aria-hidden="true" />
      <section className="software-layer" aria-labelledby="software-heading">
        <div className="layer-heading"><h3 id="software-heading">Platform</h3></div>
        {(['communication', 'execution'] as const).map(group => <div className="software-group" key={group}>
          <h4>{group === 'execution' ? 'Runtime' : 'Data'}</h4>
          <div className="software-links">
            {overview.integrations.filter(i => (i.group ?? 'communication') === group).map(i => <a
              key={i.id}
              {...detailLink}
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
