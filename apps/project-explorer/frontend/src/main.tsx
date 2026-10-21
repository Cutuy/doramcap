import { StrictMode, lazy, Suspense, useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeSlug from 'rehype-slug';
import { Activity, Cpu, Database, Radio, SlidersHorizontal, MemoryStick, ShieldAlert, Wrench, Layers, ArrowUpRight, Search, ArrowLeft, type LucideIcon } from 'lucide-react';
import generated from './.generated/catalog.json';
import { componentHref, displayName, parseRoute, resolveLink, routeHref,  type Catalog, type Component, type Kind, type Route } from './model';
import Dependencies from './Dependencies';
import Overview from './Overview';
import './style.css';

const catalog = generated as Catalog;
const DependencyGraph = lazy(() => import('./DependencyGraph'));

const icons: Record<string, LucideIcon> = { Observability: Activity, Resources: Cpu, Storage: Database, Telemetry: Radio, Configuration: SlidersHorizontal, Memory: MemoryStick, Faults: ShieldAlert, Tooling: Wrench };
function Category({ name }: { name: string }) { const Icon = icons[name] ?? Layers; return <span className="category"><Icon size={13} strokeWidth={1.6} aria-hidden="true" />{name}</span>; }
function useRoute(): Route {
  const [hash, setHash] = useState(window.location.hash);
  useEffect(() => { const update = () => setHash(window.location.hash); window.addEventListener('hashchange', update); return () => window.removeEventListener('hashchange', update); }, []);
  return useMemo(() => parseRoute(hash), [hash]);
}
function updateParams(route: Route, values: Record<string, string>) {
  const params = new URLSearchParams(route.params);
  for (const [key, value] of Object.entries(values)) { if (value) params.set(key, value); else params.delete(key); }
  window.history.pushState(null, '', routeHref(route.path, params));
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
    <div className="intro"><h1>Explore the {kind === 'apps' ? 'applications' : 'crates'}.</h1><p>{kind === 'apps' ? 'Runnable tools and demos built on the workspace’s telemetry, resource, and storage libraries.' : 'Reusable building blocks for telemetry, resource management, memory, and persistence.'}</p></div>
    <div className="toolbar"><div className="catalog-count">{visible.length} {kind === 'apps' ? 'applications' : 'crates'}</div><div className="filters"><label><span className="sr-only">Category</span><select aria-label="Category" value={category} onChange={e => updateParams(route, { category: e.target.value })}><option value="">All categories</option>{categories.map(c => <option key={c}>{c}</option>)}</select></label><label className="search"><Search size={15} aria-hidden="true" /><input aria-label="Search components" type="search" value={query} placeholder={`Search ${kind}…`} onChange={e => updateParams(route, { q: e.target.value })} /></label></div></div>
    <div className="gallery">{visible.map(c => <a className="tile" href={componentHref(c)} key={c.id}><Category name={c.category} /><h2>{displayName(c.name)}</h2><p>{c.summary}</p></a>)}</div>
    {!visible.length && <div className="empty"><h2>No matching components</h2><p>Try another search or category.</p><a href={`#/${kind}`}>Clear filters</a></div>}
  </>;
}
function Detail({ component, route }: { component: Component; route: Route }) {
  return <>
    <a className="back" href={`#/${component.kind}`}><ArrowLeft size={14} />All {component.kind === 'apps' ? 'applications' : 'crates'}</a>
    <div className="detail-heading"><div className="intro"><Category name={component.category} /><h1>{displayName(component.name)}</h1><p>{component.summary}</p><code className="identifier">{component.name}</code></div><a className="source-link" href={`${catalog.repository}/tree/${catalog.revision}/${component.directory}`} target="_blank" rel="noreferrer">Source <ArrowUpRight size={14} /></a></div>
    <div className="detail-grid"><div><DocumentView path={component.readme} route={route} /></div><aside><section className="dependency-panel"><div className="panel-title"><h2>Direct dependencies</h2><a href={`#/dependencies?focus=${encodeURIComponent(component.id)}`}>Explore <ArrowUpRight size={13} /></a></div><Dependencies catalog={catalog} focus={component.id} /></section>
      {component.packages.length > 0 && <section className="related"><h2>Cargo packages</h2>{component.packages.map(p => <a key={p} href={`#/dependencies?focus=${encodeURIComponent(p)}`}>{p} <span>→</span></a>)}</section>}
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
  useEffect(() => { document.title = `${component ? displayName(component.name) : active === 'overview' ? 'Overview' : active === 'dependencies' ? 'Dependencies' : active === 'crates' ? 'Crates' : active === 'docs' ? 'Documentation' : 'Apps'} · doramcap`; if (!route.params.has('heading')) window.scrollTo(0, 0); }, [route.path, component, active]);
  return <><a href="#main" className="skip" onClick={e => { e.preventDefault(); document.getElementById('main')?.focus(); }}>Skip to content</a><header className="site-header"><div className="shell header-inner"><a className="brand" href="#/overview">doramcap<span>.</span></a><nav aria-label="Main navigation">{[['overview', 'Overview'], ['apps', 'Apps'], ['crates', 'Crates'], ['dependencies', 'Dependencies']].map(([path, label]) => <a key={path} href={`#/${path}`} aria-current={active === path ? 'page' : undefined}>{label}</a>)}</nav><a className="repository" href={catalog.repository}>Repository <ArrowUpRight size={14} /></a></div></header>
    <main id="main" tabIndex={-1} className="shell">{component ? <Detail component={component} route={route} /> : route.path === '/' || route.path === '/overview' ? <Overview catalog={catalog} route={route} /> : route.path === '/apps' ? <Gallery kind="apps" route={route} /> : route.path === '/crates' ? <Gallery kind="crates" route={route} /> : route.path === '/dependencies' ? <Explorer route={route} /> : route.path === '/docs' ? <><a className="back" href="#/apps"><ArrowLeft size={14} />Workspace</a><DocumentView path={route.params.get('path') ?? 'README.md'} route={route} /></> : <div className="empty"><h1>Page not found</h1><a href="#/apps">Browse applications</a></div>}</main>
    <footer className="shell footer"><span>doramcap / Workspace explorer</span><a href={`${catalog.repository}/tree/${catalog.revision}`}>Source {catalog.revision.slice(0, 7)}</a><a href="#/docs?path=README.md">Workspace README</a></footer></>;
}
createRoot(document.getElementById('root')!).render(<StrictMode><App /></StrictMode>);
