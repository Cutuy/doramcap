import { StrictMode, lazy, Suspense, useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeSlug from 'rehype-slug';
import { Activity, Cpu, Database, Radio, SlidersHorizontal, MemoryStick, ShieldAlert, Wrench, Workflow, Layers, ArrowUpRight, Search, ArrowLeft, type LucideIcon } from 'lucide-react';
import generated from './.generated/catalog.json';
import { componentHref, displayName, parseRoute, resolveLink, routeHref, type Catalog, type Component, type Kind, type Route } from './model';
import Dependencies from './Dependencies';
import Overview from './Overview';
import './style.css';

const catalog = generated as Catalog;
const DependencyGraph = lazy(() => import('./DependencyGraph'));

const icons: Record<string, LucideIcon> = { Observability: Activity, Resources: Cpu, Storage: Database, Telemetry: Radio, Configuration: SlidersHorizontal, Memory: MemoryStick, Faults: ShieldAlert, Tooling: Wrench, Runtime: Workflow };
function Category({ name }: { name: string }) { const Icon = icons[name] ?? Layers; return <span className="category"><Icon size={13} strokeWidth={1.6} aria-hidden="true" />{name}</span>; }
function useRoute(): Route {
  const [hash, setHash] = useState(window.location.hash);
  useEffect(() => { const update = () => setHash(window.location.hash); window.addEventListener('hashchange', update); return () => window.removeEventListener('hashchange', update); }, []);
  return useMemo(() => parseRoute(hash), [hash]);
}
function updateParams(route: Route, values: Record<string, string>, replace = false) {
  const params = new URLSearchParams(route.params);
  for (const [key, value] of Object.entries(values)) { if (value) params.set(key, value); else params.delete(key); }
  window.history[replace ? 'replaceState' : 'pushState'](null, '', routeHref(route.path, params));
  window.dispatchEvent(new HashChangeEvent('hashchange'));
}
function DocumentView({ path, route }: { path: string; route: Route }) {
  const doc = catalog.documents[path];
  useEffect(() => {
    const anchor = route.params.get('heading');
    if (anchor) requestAnimationFrame(() => document.getElementById(anchor)?.scrollIntoView({ block: 'start' }));
  }, [path, route]);
  if (!doc) return <p role="alert">This document is not included in this build.</p>;
  return <article className="markdown"><Markdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeSlug]} components={{
    a: ({ href, children, ...props }) => { const target = resolveLink(href ?? '', path, catalog, import.meta.env.BASE_URL); return <a {...props} href={target}>{children}</a>; },
    img: ({ src, alt }) => { const target = resolveLink(typeof src === 'string' ? src : '', path, catalog, import.meta.env.BASE_URL); return <a href={target} target="_blank" rel="noreferrer" className="diagram-link"><img src={target} alt={alt ?? ''} loading="lazy" /></a>; },
    table: ({ children }) => <div className="table-scroll"><table>{children}</table></div>,
  }}>{doc.text}</Markdown></article>;
}
function Gallery({ kind, route }: { kind: Kind; route: Route }) {
  const query = route.params.get('q') ?? '';
  const category = route.params.get('category') ?? '';
  const entries = catalog.components.filter(c => c.kind === kind);
  const categories = [...new Set(entries.map(c => c.category))].sort();
  const visible = entries.filter(c => (!category || c.category === category) && `${c.name} ${displayName(c.name)} ${c.summary} ${c.category}`.toLowerCase().includes(query.toLowerCase()));
  return <>
    <div className="intro"><h1>Explore the {kind === 'apps' ? 'applications' : 'crates'}.</h1><p>{kind === 'apps' ? 'Runnable consoles, monitors and demos built on the workspace crates.' : 'Reusable building blocks for runtime, resources, memory, storage, telemetry and faults.'}</p></div>
    <div className="toolbar"><div className="catalog-count">{visible.length} {kind === 'apps' ? 'applications' : 'crates'}</div><div className="filters"><label><span className="sr-only">Category</span><select aria-label="Category" value={category} onChange={e => updateParams(route, { category: e.target.value })}><option value="">All categories</option>{categories.map(c => <option key={c}>{c}</option>)}</select></label><label className="search"><Search size={15} aria-hidden="true" /><input aria-label="Search components" type="search" value={query} placeholder={`Search ${kind}…`} onChange={e => updateParams(route, { q: e.target.value }, true)} /></label></div></div>
    <div className="gallery">{visible.map(c => <article className="tile" key={c.id}><a className="tile-main" href={componentHref(c)}><Category name={c.category} /><h2>{displayName(c.name)}</h2><p>{c.summary}</p></a>{kind === 'apps' && <div className="tile-actions"><Launch component={c} /></div>}</article>)}</div>
    {!visible.length && <div className="empty"><h2>No matching components</h2><p>Try another search or category.</p><a href={`#/${kind}`}>Clear filters</a></div>}
  </>;
}
// Console runs directly on the Docker host; the gallery never launches it.
const headlessApps = new Set(['console']);
function Launch({ component }: { component: Component }) {
  const [availability, setAvailability] = useState<'loading' | 'enabled' | 'disabled' | 'error'>('loading');
  const [ready, setReady] = useState(false);
  const [headless, setHeadless] = useState(false);
  const base = new URL(import.meta.env.BASE_URL, document.baseURI);
  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    setAvailability('loading');
    setReady(false);
    setHeadless(false);
    if (component.kind !== 'apps' || component.name === 'project-explorer' || headlessApps.has(component.name)) return;
    async function refresh() {
      try {
        const response = await fetch(new URL('api/hosting/apps', base), { signal: AbortSignal.timeout(5000) });
        if (!response.ok) throw new Error('Hosting unavailable');
        const data = await response.json();
        if (!Array.isArray(data.apps)) throw new Error('Invalid hosting response');
        const enabled = data.apps.includes(component.name);
        if (enabled) {
          const status = await fetch(new URL(`api/hosting/apps/${encodeURIComponent(component.name)}`, base), { signal: AbortSignal.timeout(5000) });
          if (!status.ok) throw new Error('Unable to read hosting status');
          const value = await status.json();
          if (active) {
            setReady(value.state === 'ready');
            setHeadless(value.headless === true);
          }
        } else if (active) { setReady(false); setHeadless(false); }
        if (active) setAvailability(enabled ? 'enabled' : 'disabled');
      } catch {
        if (active) setAvailability(previous => previous === 'enabled' ? previous : 'error');
      } finally {
        if (active) timer = setTimeout(refresh, 5000);
      }
    }
    void refresh();
    return () => { active = false; clearTimeout(timer); };
  }, [component.kind, component.name, base.href]);
  if (component.kind !== 'apps') return null;
  if (component.name === 'project-explorer') return <span className="launch-status">Current app</span>;
  if (headless || headlessApps.has(component.name)) return <span className="launch-status">Headless</span>;
  if (availability !== 'enabled') return <span className="launch-unavailable"><button className="launch-link" disabled>Start</button><span>{availability === 'loading' ? 'Checking availability…' : availability === 'error' ? 'Hosting unavailable; retrying…' : 'Not enabled on this host'}</span></span>;
  return <a className="launch-link" href={new URL(`running/${encodeURIComponent(component.name)}/`, base).href}>{ready ? 'Open' : 'Start'} <ArrowUpRight size={14} /></a>;
}
function Detail({ component, route }: { component: Component; route: Route }) {
  return <>
    <a className="back" href={`#/${component.kind}`}><ArrowLeft size={14} />All {component.kind === 'apps' ? 'applications' : 'crates'}</a>
    <div className="detail-heading"><div className="intro"><Category name={component.category} /><h1>{displayName(component.name)}</h1><p>{component.summary}</p><code className="identifier">{component.name}</code><Launch component={component} /></div><a className="source-link" href={`${catalog.repository}/tree/${catalog.revision}/${component.directory}`} target="_blank" rel="noreferrer">Source <ArrowUpRight size={14} /></a></div>
    <div className="detail-grid"><div><DocumentView path={component.readme} route={route} /></div><aside><section className="dependency-panel"><div className="panel-title"><h2>Direct dependencies</h2><a href={`#/dependencies?focus=${encodeURIComponent(component.id)}`}>Explore <ArrowUpRight size={13} /></a></div><Dependencies catalog={catalog} focus={component.id} /></section>
      {component.kind === 'apps' && component.packages.length > 0 && <section className="related"><h2>Cargo packages</h2>{component.packages.map(p => <a key={p} href={`#/dependencies?focus=${encodeURIComponent(p)}`}>{p} <span>→</span></a>)}</section>}
    </aside></div>
  </>;
}
function Explorer({ route }: { route: Route }) {
  const focus = route.params.get('focus') ?? '';
  const category = route.params.get('category') ?? '';
  const component = catalog.components.find(c => c.id === focus || c.name === focus);
  const pkg = catalog.packages.find(p => p.name === focus);
  const valid = !focus || Boolean(component || pkg);
  return <><div className="intro"><h1>Dependencies.</h1></div><div className="toolbar graph-toolbar"><label>Component <select aria-label="Dependency component" value={focus} onChange={e => updateParams(route, { focus: e.target.value })}><option value="">Whole workspace</option>{catalog.components.map(c => <option key={c.id} value={c.id}>{displayName(c.name)}</option>)}{focus && !catalog.components.some(c => c.id === focus) && <option value={focus}>{focus}</option>}</select></label>{!focus && <label>Category <select aria-label="Dependency category" value={category} onChange={e => updateParams(route, { category: e.target.value })}><option value="">All categories</option>{[...new Set(catalog.components.map(c => c.category))].sort().map(c => <option key={c}>{c}</option>)}</select></label>}<div className="dependency-kind-controls">{['build', 'dev'].map(kind => <label key={kind}><input type="checkbox" checked={route.params.get(kind) === '1'} onChange={e => updateParams(route, { [kind]: e.target.checked ? '1' : '' })} />{kind === 'build' ? 'Build' : 'Dev'}</label>)}</div></div>{valid ? <Suspense fallback={<p role="status">Loading dependency graph…</p>}><DependencyGraph catalog={catalog} focus={focus} category={category} build={route.params.get('build') === '1'} dev={route.params.get('dev') === '1'} /></Suspense> : <p role="alert">Unknown component. <a href="#/dependencies">Show workspace</a>.</p>}</>;

}
function App() {
  const route = useRoute();
  const component = catalog.components.find(c => `/${c.id}` === route.path);
  const active = route.path === '/' || route.path === '/overview' ? 'overview' : route.path.startsWith('/crates') ? 'crates' : route.path === '/dependencies' ? 'dependencies' : route.path.startsWith('/docs') ? 'docs' : 'apps';
  useEffect(() => { document.title = `${component ? displayName(component.name) : active === 'overview' ? 'Overview' : active === 'dependencies' ? 'Dependencies' : active === 'crates' ? 'Crates' : active === 'docs' ? 'Documentation' : 'Apps'} · doramcap`; if (!route.params.has('heading')) window.scrollTo(0, 0); }, [route.path, route.params.get('path'), component, active]);
  return <><a href="#main" className="skip" onClick={e => { e.preventDefault(); document.getElementById('main')?.focus(); }}>Skip to content</a><header className="site-header"><div className="shell header-inner"><a className="brand" href="#/overview">doramcap<span>.</span></a><nav aria-label="Main navigation">{[['overview', 'Overview'], ['apps', 'Apps'], ['crates', 'Crates'], ['dependencies', 'Dependencies']].map(([path, label]) => <a key={path} href={`#/${path}`} aria-current={active === path ? 'page' : undefined}>{label}</a>)}</nav><a className="repository" href={catalog.repository}>Repository <ArrowUpRight size={14} /></a></div></header>
    <main id="main" tabIndex={-1} className="shell">{component ? <Detail component={component} route={route} /> : route.path === '/' || route.path === '/overview' ? <Overview catalog={catalog} route={route} /> : route.path === '/apps' ? <Gallery kind="apps" route={route} /> : route.path === '/crates' ? <Gallery kind="crates" route={route} /> : route.path === '/dependencies' ? <Explorer route={route} /> : route.path === '/docs' ? <><a className="back" href="#/overview"><ArrowLeft size={14} />Overview</a><DocumentView path={route.params.get('path') ?? 'README.md'} route={route} /></> : <div className="empty"><h1>Page not found</h1><a href="#/apps">Browse applications</a></div>}</main>
    <footer className="shell footer"><span>doramcap project explorer</span><a href={`${catalog.repository}/tree/${catalog.revision}`}>Source {catalog.revision.slice(0, 7)}</a><a href="#/docs?path=README.md">Workspace README</a></footer></>;
}
createRoot(document.getElementById('root')!).render(<StrictMode><App /></StrictMode>);
