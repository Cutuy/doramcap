import { lazy, Suspense, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type PointerEvent } from 'react';
import { AnimatePresence, MotionConfig, animate, motion, useInView, useMotionValueEvent, useReducedMotion, useScroll, useTransform } from 'motion/react';
import { Activity, ArrowUpRight, Box, Cpu, Database, Layers, MemoryStick, Radio, Rows3, ShieldAlert, SlidersHorizontal, Workflow, X } from 'lucide-react';
import data from './.generated/overview.json';
import { componentHref, displayName, href, navigate, routeHref, type Catalog, type Component, type Route } from './model';
import type { Capability, Flow, Integration, Mapping, OverviewData } from './overview-model';
import StackSchematic from './StackSchematic';
import { flowLabels, stackNodes, stackView, type StackNode, type StackView } from './stack';

const overview = data as OverviewData;
const nodes = stackNodes(overview);
const chapters = overview.useCases;
const StoryCanvas = lazy(() => import('./landing/StoryCanvas'));
const icons: Record<string, typeof Activity> = { Resources: Cpu, Memory: MemoryStick, Observability: Activity, Storage: Database, Faults: ShieldAlert, Configuration: SlidersHorizontal, Telemetry: Radio, Runtime: Workflow };
const statusLabels: Record<string, string> = { planned: 'Planned', optional: 'Optional', 'platform-dependent': 'Platform-dependent' };
const ease = [.22, 1, .36, 1] as const;
// Every flow from every use case: the hero shows the whole platform in motion.
const heroFlows = [...new Map(chapters.flatMap(u => u.flows).map(f => [`${f.from}>${f.to}`, f])).values()];
const pad = (n: number) => String(n).padStart(2, '0');

/** Count up to `value` the first time it scrolls into view. */
function Count({ value }: { value: number }) {
  const ref = useRef<HTMLSpanElement>(null);
  const seen = useInView(ref, { once: true });
  const still = useReducedMotion();
  useEffect(() => {
    if (!seen || still) return;
    const run = animate(0, value, { duration: 1.4, ease, onUpdate: v => { if (ref.current) ref.current.textContent = String(Math.round(v)); } });
    return () => run.stop();
  }, [seen, still, value]);
  return <span ref={ref}>{still ? value : 0}</span>;
}

function currentParams() { return new URLSearchParams(window.location.search); }
function go(params: URLSearchParams, push: boolean) { navigate(routeHref('/overview', params), !push); }
function selectionOf(node: StackNode): [string, string] | null {
  if (!node.select) return null;
  return 'capability' in node.select ? ['capability', node.select.capability] : ['integration', node.select.integration];
}

export default function Overview({ catalog, route }: { catalog: Catalog; route: Route }) {
  // Full-bleed landing layout; set before child effects so the scene measures the final layout.
  useLayoutEffect(() => { document.documentElement.dataset.landing = ''; return () => { delete document.documentElement.dataset.landing; }; }, []);
  return <MotionConfig reducedMotion="user"><div className="landing">
    <Story catalog={catalog} route={route} />
    <Foundation catalog={catalog} />
    <Platform />
    <Explore catalog={catalog} />
  </div></MotionConfig>;
}

function ComponentLinks({ item, catalog }: { item: Mapping; catalog: Catalog }) {
  const entries = item.components.map(id => catalog.components.find(c => c.id === id)).filter((c): c is Component => Boolean(c));
  return <div className="chips">{entries.map(c => <a key={c.id} href={componentHref(c)}><span className={`chip-kind kind-${c.kind}`} />{displayName(c.name)}</a>)}</div>;
}

function Story({ catalog, route }: { catalog: Catalog; route: Route }) {
  const story = useRef<HTMLElement>(null);
  const { scrollYProgress } = useScroll({ target: story, offset: ['start start', 'end end'] });
  const progress = useTransform(scrollYProgress, v => v * chapters.length);
  const [chapter, setChapter] = useState(-1);
  const [webgl, setWebgl] = useState(true);
  const ready = useRef(false);
  useMotionValueEvent(progress, 'change', p => setChapter(Math.max(-1, Math.min(chapters.length - 1, Math.round(p) - 1))));

  const capability = overview.capabilities.find(c => c.id === route.params.get('capability'));
  const integration = overview.integrations.find(i => i.id === route.params.get('integration'));
  const selected: Capability | Integration | undefined = capability ?? integration;
  const useCase = chapter >= 0 ? chapters[chapter] : undefined;
  const flat = route.params.get('view') === '2d' || !webgl;
  const view: StackView = useMemo(() => useCase || selected ? stackView(overview, useCase, capability, integration, heroFlows) : { states: {}, flows: heroFlows }, [useCase, selected, capability, integration]);
  const selectedNode = capability ? capability.id : integration?.id ?? null;

  // Chapter i sits at progress i + 1; measure the scroll range useScroll maps, since vh, svh and innerHeight can differ.
  const scrollTo = useCallback((index: number, behavior: ScrollBehavior = 'smooth') => {
    const el = story.current;
    if (!el) return;
    window.scrollTo({ top: el.offsetTop + (index + 1) / chapters.length * (el.offsetHeight - window.innerHeight), behavior });
  }, []);
  // Deep links open their chapter; wait for the router's scroll reset first.
  useEffect(() => {
    const index = chapters.findIndex(u => u.id === route.params.get('useCase'));
    requestAnimationFrame(() => requestAnimationFrame(() => { if (index >= 0) scrollTo(index, 'instant'); ready.current = true; }));
  }, []);
  // Navigation to another use case, or to the bare overview, scrolls the story to match.
  const shown = useRef('');
  shown.current = useCase?.id ?? '';
  useEffect(() => {
    const follow = () => {
      const id = currentParams().get('useCase') ?? '';
      if (ready.current && id !== shown.current) scrollTo(chapters.findIndex(u => u.id === id));
    };
    window.addEventListener('locationchange', follow);
    window.addEventListener('popstate', follow);
    return () => { window.removeEventListener('locationchange', follow); window.removeEventListener('popstate', follow); };
  }, [scrollTo]);
  // Scrolling to a new chapter records it in the URL and closes any open selection.
  useEffect(() => {
    if (!ready.current) return;
    const params = currentParams(), id = useCase?.id ?? '';
    if ((params.get('useCase') ?? '') === id) return;
    const open = params.has('capability') || params.has('integration');
    if (id) params.set('useCase', id); else params.delete('useCase');
    params.delete('capability'); params.delete('integration');
    if (open) go(params, false); else window.history.replaceState(null, '', routeHref('/overview', params));
  }, [useCase]);

  const select = useCallback((node: StackNode | null) => {
    const params = currentParams(), next = node && selectionOf(node);
    const same = next && params.get(next[0]) === next[1];
    if (!next && !params.has('capability') && !params.has('integration')) return;
    params.delete('capability'); params.delete('integration');
    if (next && !same) params.set(...next);
    go(params, true);
  }, []);
  useEffect(() => {
    if (!selected) return;
    const close = (e: KeyboardEvent) => { if (e.key === 'Escape') select(null); };
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, [selected, select]);
  const hrefFor = useCallback((node: StackNode) => {
    const params = currentParams(), next = selectionOf(node);
    if (!next) return undefined;
    const same = params.get(next[0]) === next[1];
    params.delete('capability'); params.delete('integration');
    if (!same) params.set(...next);
    return routeHref('/overview', params);
  }, [route]);
  const describe = useCallback((node: StackNode) => node.id === 'middleware' ? 'Your Docker Compose apps, unchanged. Orchestrated and recorded by doramcap.'
    : (overview.capabilities.find(c => c.id === node.id) ?? overview.integrations.find(i => i.id === node.id))?.summary ?? '', []);
  const viewHref = (v: '3d' | '2d') => { const params = currentParams(); params.set('view', v); return routeHref('/overview', params); };
  const unavailable = useCallback(() => setWebgl(false), []);
  const counts = { crates: catalog.components.filter(c => c.kind === 'crates').length, apps: catalog.components.filter(c => c.kind === 'apps').length, integrations: overview.integrations.length };
  const kinds = [...new Set(view.flows.map(f => f.kind))] as Flow['kind'][];
  const bar = useTransform(scrollYProgress, [0, 1], [0, 1]);

  return <section ref={story} className="story" style={{ height: `${(chapters.length + 1) * 100}vh` }} aria-label="Platform story">
    <div className="story-viewport">
      <div className={`story-stage${flat ? ' flat' : ''}`}>
        {flat
          ? <div className="story-schematic"><StackSchematic nodes={nodes} integrations={overview.integrations} view={view} hrefFor={hrefFor} /></div>
          : <Suspense fallback={null}><StoryCanvas nodes={nodes} integrations={overview.integrations} view={view} selected={selectedNode} progress={progress} describe={describe} onSelect={select} onUnavailable={unavailable} /></Suspense>}
      </div>
      <div className="story-vignette" aria-hidden="true" />
      <motion.div className="story-progress" style={{ scaleX: bar }} aria-hidden="true" />

      <div className="story-copy-wrap">
        <AnimatePresence mode="wait">
          {useCase ? <motion.div key={useCase.id} className="story-copy"
            initial={{ opacity: 0, y: 28, filter: 'blur(6px)' }} animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }} exit={{ opacity: 0, y: -20, filter: 'blur(6px)' }} transition={{ duration: .55, ease }}>
            <div className="eyebrow">Use case <b>{pad(chapter + 1)}</b> / {pad(chapters.length)}</div>
            <h2>{useCase.title}</h2>
            <p>{useCase.summary}</p>
            <ComponentLinks item={useCase} catalog={catalog} />
            <div className="flow-legend">{kinds.map(k => <span key={k} className={`legend-${k}`}><b aria-hidden="true" />{flowLabels[k]}</span>)}</div>
          </motion.div> : <motion.div key="hero" className="story-copy hero-copy"
            initial={{ opacity: 0, y: 24 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -24, filter: 'blur(6px)' }} transition={{ duration: .7, ease }}>
            <div className="eyebrow pulse">Source-available runtime · Rust · Dora · MCAP</div>
            <h1 aria-label={overview.title}>{overview.title.split(' ').map((word, i, all) => <motion.span key={i} className={`word${i >= all.length - 2 ? ' accent' : ''}`} aria-hidden="true" initial={{ opacity: 0, y: '0.4em', filter: 'blur(8px)' }} animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }} transition={{ duration: .8, delay: .15 + i * .08, ease }}>{word} </motion.span>)}</h1>
            <p>{overview.summary}</p>
            <dl className="hero-stats">
              <div><dt><Count value={counts.crates} /></dt><dd>library crates</dd></div>
              <div><dt><Count value={counts.apps} /></dt><dd>runnable apps</dd></div>
              <div><dt><Count value={counts.integrations} /></dt><dd>platform integrations</dd></div>
            </dl>
            <a className="scroll-cue" href={routeHref('/overview', new URLSearchParams({ useCase: chapters[0].id }))} onClick={e => { e.preventDefault(); scrollTo(0); }}><span />Explore use cases</a>
          </motion.div>}
        </AnimatePresence>
      </div>

      <nav className="story-rail" aria-label="Use cases">
        {[undefined, ...chapters].map((u, i) => <a key={u?.id ?? 'intro'} href={routeHref('/overview', new URLSearchParams(u ? { useCase: u.id } : {}))} aria-current={chapter === i - 1 ? 'step' : undefined}
          onClick={e => { e.preventDefault(); if (u) scrollTo(i - 1); else window.scrollTo({ top: story.current?.offsetTop ?? 0, behavior: 'smooth' }); }}>
          <span className="rail-index">{pad(i)}</span><span className="rail-label">{u?.title ?? 'Platform'}</span>
        </a>)}
      </nav>
      <nav className="view-switch story-switch" aria-label="Stack view">{(['3d', '2d'] as const).map(v => { const Icon = v === '3d' ? Box : Rows3; return <a key={v} href={viewHref(v)} aria-current={(flat ? '2d' : '3d') === v ? 'page' : undefined}><Icon size={14} strokeWidth={1.7} aria-hidden="true" />{v.toUpperCase()}</a>; })}</nav>

      <AnimatePresence>
        {selected && <motion.aside key={selected.id} className="story-drawer" aria-live="polite"
          initial={{ opacity: 0, x: 40 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: 40 }} transition={{ duration: .4, ease }}>
          <button className="drawer-close" type="button" onClick={() => select(null)} aria-label="Close"><X size={16} /></button>
          <div className="eyebrow">{capability ? 'Capability' : 'Integration'}{statusLabels[selected.status] && <> · {statusLabels[selected.status]}</>}</div>
          <h3>{selected.title}</h3>
          <p>{selected.summary}</p>
          <ComponentLinks item={selected} catalog={catalog} />
          {integration && <a className="upstream" href={integration.url} target="_blank" rel="noreferrer">Upstream project <ArrowUpRight size={14} /></a>}
        </motion.aside>}
      </AnimatePresence>
    </div>
  </section>;
}

const reveal = { initial: { opacity: 0, y: 32 }, whileInView: { opacity: 1, y: 0 }, viewport: { once: true, margin: '-80px' } };

function SectionHead({ eyebrow, title }: { eyebrow: string; title: string }) {
  return <motion.header className="section-head" {...reveal} transition={{ duration: .7, ease }}><div className="eyebrow">{eyebrow}</div><h2>{title}</h2></motion.header>;
}

function Foundation({ catalog }: { catalog: Catalog }) {
  // A cursor-following spotlight; position lives in CSS variables, so no re-render.
  const spotlight = (e: PointerEvent<HTMLElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    e.currentTarget.style.setProperty('--mx', `${e.clientX - r.left}px`);
    e.currentTarget.style.setProperty('--my', `${e.clientY - r.top}px`);
  };
  return <section className="landing-section">
    <SectionHead eyebrow="Foundation" title="The doramcap foundation" />
    <div className="bento">{overview.capabilities.map((c, i) => {
      const Icon = icons[c.category] ?? Layers;
      return <motion.article key={c.id} className={`bento-card${i === 0 ? ' wide' : ''}`} onPointerMove={spotlight} {...reveal} transition={{ duration: .6, delay: (i % 4) * .07, ease }}>
        <div className="bento-head"><Icon size={18} strokeWidth={1.5} aria-hidden="true" /><span>{c.category}</span></div>
        <h3>{c.title}</h3>
        <p>{c.summary}</p>
        <ComponentLinks item={c} catalog={catalog} />
      </motion.article>;
    })}</div>
  </section>;
}

function Platform() {
  const groups = [['execution', 'Runtime'], ['communication', 'Data']] as const;
  return <section className="landing-section">
    <SectionHead eyebrow="Platform" title="Built on Linux, CUDA, Dora and MCAP" />
    <div className="platform-grid">
      {groups.map(([group, label]) => <motion.div key={group} className="platform-col" {...reveal} transition={{ duration: .6, ease }}>
        <h3>{label}</h3>
        {overview.integrations.filter(i => (i.group ?? 'communication') === group).map(i => <a key={i.id} className="integration" href={i.url} target="_blank" rel="noreferrer">
          <span className="integration-title">{i.title}{statusLabels[i.status] && <small>{statusLabels[i.status]}</small>}</span>
          <span className="integration-summary">{i.summary}</span>
          <ArrowUpRight size={15} aria-hidden="true" />
        </a>)}
      </motion.div>)}
    </div>
    <motion.dl className="reach" {...reveal} transition={{ duration: .6, ease }}>
      <div><dt>Compute</dt><dd>{overview.platforms.join(' · ')}</dd></div>
      <div><dt>Embodiments</dt><dd>{overview.embodiments.join(' · ')}</dd></div>
    </motion.dl>
  </section>;
}

function Explore({ catalog }: { catalog: Catalog }) {
  const count = (kind: 'apps' | 'crates') => catalog.components.filter(c => c.kind === kind).length;
  const cards = [
    { href: href('/apps'), value: count('apps'), label: 'Apps', text: 'Runnable consoles, monitors and demos built on the workspace crates.' },
    { href: href('/crates'), value: count('crates'), label: 'Crates', text: 'Reusable building blocks for runtime, resources, memory, storage, telemetry and faults.' },
    { href: href('/dependencies'), value: catalog.dependencies.length, label: 'Dependencies', text: 'The Cargo dependency graph across the workspace.' },
  ];
  return <section className="landing-section explore">
    <SectionHead eyebrow="Workspace" title="Explore the workspace" />
    <div className="explore-grid">{cards.map((c, i) => <motion.a key={c.href} href={c.href} className="explore-card" {...reveal} transition={{ duration: .6, delay: i * .08, ease }}>
      <span className="explore-value"><Count value={c.value} /></span>
      <span className="explore-label">{c.label} <ArrowUpRight size={18} aria-hidden="true" /></span>
      <span className="explore-text">{c.text}</span>
    </motion.a>)}</div>
    <motion.a className="repo-line" href={catalog.repository} {...reveal} transition={{ duration: .6, ease }}>Source, documentation and roadmap on GitHub <ArrowUpRight size={16} aria-hidden="true" /></motion.a>
  </section>;
}
