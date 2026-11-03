import { StrictMode, lazy, Suspense, useEffect, useMemo, useRef, useState, type PointerEvent, type ReactNode } from 'react';
import { motion } from 'motion/react';
import { createRoot } from 'react-dom/client';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeSlug from 'rehype-slug';
import { Activity, Cpu, Database, Radio, SlidersHorizontal, MemoryStick, ShieldAlert, Wrench, Workflow, Layers, ArrowUpRight, Search, ArrowLeft, Copy, Check, Sun, Moon, type LucideIcon } from 'lucide-react';
import generated from './.generated/catalog.json';
import { componentHref, displayName, href, isAppLink, navigate, parseRoute, resolveLink, routeHref, type Catalog, type Component, type Kind, type Route } from './model';
import Dependencies from './Dependencies';
import Overview from './Overview';
import { pinTheme, useTheme } from './theme';
import './style.css';

const catalog = generated as Catalog;
const DependencyGraph = lazy(() => import('./DependencyGraph'));

const icons: Record<string, LucideIcon> = { Observability: Activity, Resources: Cpu, Storage: Database, Telemetry: Radio, Configuration: SlidersHorizontal, Memory: MemoryStick, Faults: ShieldAlert, Tooling: Wrench, Runtime: Workflow };
function Category({ name }: { name: string }) { const Icon = icons[name] ?? Layers; return <span className="category"><Icon size={13} strokeWidth={1.6} aria-hidden="true" />{name}</span>; }
/** Cursor-following spotlight; the position lives in CSS variables, so no re-render. */
function spotlight(e: PointerEvent<HTMLElement>) {
  const r = e.currentTarget.getBoundingClientRect();
  e.currentTarget.style.setProperty('--mx', `${e.clientX - r.left}px`);
  e.currentTarget.style.setProperty('--my', `${e.clientY - r.top}px`);
}
function ThemeToggle() {
  const theme = useTheme(), next = theme === 'dark' ? 'light' : 'dark';
  const Icon = theme === 'dark' ? Sun : Moon;
  return <button type="button" className="theme-toggle" onClick={() => pinTheme(next)} aria-label={`Switch to ${next} theme`} title={`Switch to ${next} theme`}><Icon size={15} strokeWidth={1.7} aria-hidden="true" /></button>;
}
function useRoute(): Route {
  const [location, setLocation] = useState(window.location.pathname + window.location.search);
  useEffect(() => {
    const update = () => setLocation(window.location.pathname + window.location.search);
    // Handle internal links in the client; modified clicks and new-tab targets keep browser behavior.
    const click = (e: MouseEvent) => {
      if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
      const anchor = (e.target as Element | null)?.closest('a');
      const target = anchor?.getAttribute('href');
      if (!anchor || !target || target.startsWith('#') || (anchor.getAttribute('target') ?? '_self') !== '_self' || anchor.hasAttribute('download')) return;
      const url = new URL(target, window.location.href);
      if (!isAppLink(url)) return;
      e.preventDefault();
      if (url.pathname + url.search !== window.location.pathname + window.location.search) navigate(url.pathname + url.search);
    };
    window.addEventListener('popstate', update);
    window.addEventListener('locationchange', update);
    document.addEventListener('click', click);
    return () => { window.removeEventListener('popstate', update); window.removeEventListener('locationchange', update); document.removeEventListener('click', click); };
  }, []);
  return useMemo(() => { const [pathname, search = ''] = location.split('?'); return parseRoute({ pathname, search: search && `?${search}` }); }, [location]);
}
function updateParams(route: Route, values: Record<string, string>, replace = false) {
  const params = new URLSearchParams(route.params);
  for (const [key, value] of Object.entries(values)) { if (value) params.set(key, value); else params.delete(key); }
  navigate(routeHref(route.path, params), replace);
}
/** A code block with a copy control; falls back to selecting the text when the clipboard is unavailable. */
function CodeBlock({ children }: { children: ReactNode }) {
  const code = useRef<HTMLPreElement>(null);
  const [copied, setCopied] = useState(false);
  useEffect(() => { if (!copied) return; const t = setTimeout(() => setCopied(false), 1600); return () => clearTimeout(t); }, [copied]);
  async function copy() {
    const text = code.current?.textContent?.replace(/\n$/, '') ?? '';
    try { await navigator.clipboard.writeText(text); setCopied(true); }
    catch { const range = document.createRange(); range.selectNodeContents(code.current!); const selection = window.getSelection(); selection?.removeAllRanges(); selection?.addRange(range); }
  }
  return <div className="code-block">
    <pre ref={code}>{children}</pre>
    <button type="button" className={`copy-code${copied ? ' copied' : ''}`} onClick={copy} aria-label={copied ? 'Copied' : 'Copy code'}>
      {copied ? <Check size={14} aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />}<span>{copied ? 'Copied' : 'Copy'}</span>
    </button>
  </div>;
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
    pre: ({ children }) => <CodeBlock>{children}</CodeBlock>,
  }}>{doc.text}</Markdown></article>;
}
function Gallery({ kind, route }: { kind: Kind; route: Route }) {
  const query = route.params.get('q') ?? '';
  const entries = catalog.components.filter(c => c.kind === kind);
  const visible = entries.filter(c => `${c.name} ${displayName(c.name)} ${c.summary} ${c.category}`.toLowerCase().includes(query.toLowerCase()));
  return <>
    <div className="intro page-intro"><div className="eyebrow">Workspace · {entries.length} {kind}</div><h1>Explore the {kind === 'apps' ? 'applications' : 'crates'}.</h1><p>{kind === 'apps' ? 'Runnable consoles, monitors and demos built on the workspace crates.' : 'Reusable building blocks for runtime, resources, memory, storage, telemetry and faults.'}</p></div>
    <div className="toolbar"><div className="catalog-count">{visible.length} {kind === 'apps' ? 'applications' : 'crates'}</div><div className="filters"><label className="search"><Search size={15} aria-hidden="true" /><input aria-label="Search components" type="search" value={query} placeholder={`Search ${kind}…`} onChange={e => updateParams(route, { q: e.target.value }, true)} /></label></div></div>
    <div className="gallery">{visible.map((c, i) => <motion.article className="tile" key={c.id} onPointerMove={spotlight} initial={{ opacity: 0, y: 18 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: .45, delay: Math.min(i, 12) * .035, ease: [.22, 1, .36, 1] }}><a className="tile-main" href={componentHref(c)}><Category name={c.category} /><h2>{displayName(c.name)}</h2><p>{c.summary}</p></a>{kind === 'apps' && <div className="tile-actions"><Launch component={c} /></div>}</motion.article>)}</div>
    {!visible.length && <div className="empty"><h2>No matching components</h2><p>Try another search.</p><a href={href(`/${kind}`)}>Clear search</a></div>}
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
    <a className="back" href={href(`/${component.kind}`)}><ArrowLeft size={14} />All {component.kind === 'apps' ? 'applications' : 'crates'}</a>
    <div className="detail-heading"><div className="intro page-intro"><Category name={component.category} /><h1>{displayName(component.name)}</h1><p>{component.summary}</p><code className="identifier">{component.name}</code><Launch component={component} /></div><a className="source-link" href={`${catalog.repository}/tree/${catalog.revision}/${component.directory}`} target="_blank" rel="noreferrer">Source <ArrowUpRight size={14} /></a></div>
    <div className="detail-grid"><div><DocumentView path={component.readme} route={route} /></div><aside><section className="dependency-panel"><div className="panel-title"><h2>Direct dependencies</h2><a href={href('/dependencies', { focus: component.id })}>Explore <ArrowUpRight size={13} /></a></div><Dependencies catalog={catalog} focus={component.id} /></section>
      {component.kind === 'apps' && component.packages.length > 0 && <section className="related"><h2>Cargo packages</h2>{component.packages.map(p => <a key={p} href={href('/dependencies', { focus: p })}>{p} <span>→</span></a>)}</section>}
    </aside></div>
  </>;
}
function Explorer({ route }: { route: Route }) {
  const focus = route.params.get('focus') ?? '';
  const component = catalog.components.find(c => c.id === focus || c.name === focus);
  const pkg = catalog.packages.find(p => p.name === focus);
  const valid = !focus || Boolean(component || pkg);
  return <><div className="intro page-intro"><div className="eyebrow">Workspace · {catalog.packages.length} packages</div><h1>Dependencies.</h1></div><div className="toolbar graph-toolbar"><label>Component <select aria-label="Dependency component" value={focus} onChange={e => updateParams(route, { focus: e.target.value })}><option value="">Whole workspace</option>{catalog.components.map(c => <option key={c.id} value={c.id}>{displayName(c.name)}</option>)}{focus && !catalog.components.some(c => c.id === focus) && <option value={focus}>{focus}</option>}</select></label><div className="dependency-kind-controls">{['build', 'dev'].map(kind => <label key={kind}><input type="checkbox" checked={route.params.get(kind) === '1'} onChange={e => updateParams(route, { [kind]: e.target.checked ? '1' : '' })} />{kind === 'build' ? 'Build' : 'Dev'}</label>)}</div></div>{valid ? <Suspense fallback={<p role="status">Loading dependency graph…</p>}><DependencyGraph catalog={catalog} focus={focus} build={route.params.get('build') === '1'} dev={route.params.get('dev') === '1'} /></Suspense> : <p role="alert">Unknown component. <a href={href('/dependencies')}>Show workspace</a>.</p>}</>;

}
function App() {
  const route = useRoute();
  const component = catalog.components.find(c => `/${c.id}` === route.path);
  const active = route.path === '/' || route.path === '/overview' ? 'overview' : route.path.startsWith('/crates') ? 'crates' : route.path === '/dependencies' ? 'dependencies' : route.path.startsWith('/docs') ? 'docs' : 'apps';
  useEffect(() => { document.title = `${component ? displayName(component.name) : active === 'overview' ? 'Overview' : active === 'dependencies' ? 'Dependencies' : active === 'crates' ? 'Crates' : active === 'docs' ? 'Documentation' : 'Apps'} · doramcap`; if (!route.params.has('heading')) window.scrollTo(0, 0); }, [route.path, route.params.get('path'), component, active]);
  return <><a href="#main" className="skip" onClick={e => { e.preventDefault(); document.getElementById('main')?.focus(); }}>Skip to content</a><header className="site-header"><div className="shell header-inner"><a className="brand" href={href('/overview')}>doramcap<span>.</span></a><nav aria-label="Main navigation">{[['overview', 'Overview'], ['apps', 'Apps'], ['crates', 'Crates'], ['dependencies', 'Dependencies']].map(([path, label]) => <a key={path} href={href(`/${path}`)} aria-current={active === path ? 'page' : undefined}>{label}</a>)}</nav><div className="header-tools"><ThemeToggle /><a className="repository" href={catalog.repository}>Repository <ArrowUpRight size={14} /></a></div></div></header>
    <main id="main" tabIndex={-1} className="shell">{component ? <Detail component={component} route={route} /> : route.path === '/' || route.path === '/overview' ? <Overview catalog={catalog} route={route} /> : route.path === '/apps' ? <Gallery kind="apps" route={route} /> : route.path === '/crates' ? <Gallery kind="crates" route={route} /> : route.path === '/dependencies' ? <Explorer route={route} /> : route.path === '/docs' ? <><a className="back" href={href('/overview')}><ArrowLeft size={14} />Overview</a><DocumentView path={route.params.get('path') ?? 'README.md'} route={route} /></> : <div className="empty"><h1>Page not found</h1><a href={href('/apps')}>Browse applications</a></div>}</main>
    <footer className="shell footer"><span>doramcap project explorer</span><a href={`${catalog.repository}/tree/${catalog.revision}`}>Source {catalog.revision.slice(0, 7)}</a><a href={href('/docs', { path: 'README.md' })}>Workspace README</a></footer></>;
}
// Links shared before clean URLs used `#/path?query`; rewrite them once at startup.
if (window.location.hash.startsWith('#/')) {
  const [path, query = ''] = window.location.hash.slice(1).split('?');
  window.history.replaceState(null, '', routeHref(path, new URLSearchParams(query)));
}
createRoot(document.getElementById('root')!).render(<StrictMode><App /></StrictMode>);
