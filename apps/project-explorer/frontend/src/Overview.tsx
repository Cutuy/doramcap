import { lazy, Suspense, useCallback, useMemo, useState } from 'react';
import data from './.generated/overview.json';
import { componentHref, displayName, routeHref, type Catalog, type Route } from './model';
import type { OverviewData, Mapping } from './overview-model';
import StackSchematic from './StackSchematic';
import { flowLabels, stackNodes, stackView, type StackNode } from './stack';
const overview = data as OverviewData;
const nodes = stackNodes(overview);
const Stack3D = lazy(() => import('./Stack3D'));
const statusLabels: Record<string, string> = { 'building-blocks': '', planned: 'Planned', integrated: '', optional: 'Optional', 'platform-dependent': 'Platform-dependent', illustrative: '', validated: 'Validated use case' };

export default function Overview({ catalog, route }: { catalog: Catalog; route: Route }) {
  const useCaseId = route.params.get('useCase') ?? overview.useCases[0].id;
  const capabilityId = route.params.get('capability');
  const integrationId = route.params.get('integration');
  const [webgl, setWebgl] = useState(true);
  const view = route.params.get('view') === '2d' || !webgl || (!route.params.has('view') && window.innerWidth < 760) ? '2d' : '3d';
  const useCase = overview.useCases.find(u => u.id === useCaseId);
  const capability = overview.capabilities.find(c => c.id === capabilityId);
  const integration = overview.integrations.find(i => i.id === integrationId);
  const selected = integration ?? capability ?? useCase;
  const invalid = (useCaseId && !useCase) || (capabilityId && !capability) || (integrationId && !integration);
  const viewParam = route.params.get('view') ?? '';
  // Links keep the selected use case and an explicit view choice.
  const link = useCallback((values: Record<string, string>) => {
    const params = new URLSearchParams(Object.entries({ ...(useCase ? { useCase: useCase.id } : {}), ...(viewParam ? { view: viewParam } : {}), ...values }).filter(([, v]) => v));
    return routeHref('/overview', params);
  }, [useCase, viewParam]);
  const hrefFor = useCallback((node: StackNode) => node.select && link(node.select), [link]);
  const viewLink = (v: '3d' | '2d') => routeHref('/overview', new URLSearchParams([...[...route.params].filter(([k]) => k !== 'view'), ['view', v]]));
  const stack = useMemo(() => stackView(overview, useCase, capability, integration), [useCase, capability, integration]);
  const unavailable = useCallback(() => setWebgl(false), []);
  const kinds = [...new Set(stack.flows.map(f => f.kind))];

  function components(item: Mapping, kind: 'apps' | 'crates') {
    const entries = item.components.map(id => catalog.components.find(c => c.id === id)!).filter(c => c.kind === kind);
    return entries.length ? <section><h3>{kind === 'apps' ? 'Apps' : 'Crates'}</h3><div className="mapping-links">{entries.map(c => <a key={c.id} href={componentHref(c)}>{displayName(c.name)} <span aria-hidden="true">→</span></a>)}</div></section> : null;
  }
  return <div className="overview">
    <div className="intro"><h1>{overview.title}</h1></div>
    <section className="scenario-picker" aria-labelledby="scenario-heading"><h2 id="scenario-heading">Explore use cases</h2>
      <nav className="scenario-tabs" aria-label="Use cases">{overview.useCases.map(u => <a key={u.id} href={link({ useCase: u.id })} aria-current={useCase?.id === u.id && !capability && !integration ? 'page' : useCase?.id === u.id ? 'true' : undefined}>{u.title}</a>)}</nav>
    </section>
    {invalid && <p role="alert">This selection is not in the current overview. <a href="#/overview">Show the first use case</a>.</p>}
    <div className="overview-body">
      {selected && <section className="mapping-detail" aria-live="polite">
        <div className="mapping-heading">{statusLabels[selected.status] && <div className="eyebrow">{statusLabels[selected.status]}</div>}<h2>{selected.title}</h2><p>{selected.summary}</p>{integration && <a href={integration.url} target="_blank" rel="noreferrer">Upstream project ↗</a>}</div>
        <div className="mapping-components">{components(selected, 'apps')}{components(selected, 'crates')}</div>
      </section>}
      <section className="stack-viewer" aria-label="Application platform stack">
        <div className="viewer-bar"><span className="eyebrow">Application platform</span>
          <nav className="view-switch" aria-label="Stack view">{(['3d', '2d'] as const).map(v => <a key={v} href={viewLink(v)} aria-current={view === v ? 'page' : undefined}>{v.toUpperCase()}</a>)}</nav>
        </div>
        <div className="viewer-stage">{view === '3d'
          ? <Suspense fallback={<p className="loading" role="status">Loading 3D view…</p>}><Stack3D nodes={nodes} integrations={overview.integrations} view={stack} hrefFor={hrefFor} onUnavailable={unavailable} /></Suspense>
          : <StackSchematic nodes={nodes} integrations={overview.integrations} view={stack} hrefFor={hrefFor} />}</div>
        <div className="viewer-foot">
          <div className="flow-legend">{kinds.map(k => <span key={k} className={`legend-${k}`}><b aria-hidden="true" />{flowLabels[k]}</span>)}</div>
          <p><span>Embodiments</span>{overview.embodiments.join(' · ')}</p>
        </div>
      </section>
    </div>
  </div>;
}
