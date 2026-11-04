// Renders the root README's hero and use-case cards as animated SVGs from the overview manifest and catalog.
// Mirrors the landing's StoryScene: same stack layout, camera angles and flow curves, projected in perspective.
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Catalog } from '../src/model';
import type { Flow, OverviewData } from '../src/overview-model';
import { flowLabels, pairOffset, stackNodes, type Tier } from '../src/stack';

type Theme = 'dark' | 'light';
interface V3 { x: number; y: number; z: number; }
interface P2 { x: number; y: number; }

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..');
const out = resolve(root, 'docs/readme'), fontDir = resolve(out, 'fonts');
const generated = resolve(root, 'apps/project-explorer/frontend/src/.generated');
const overview: OverviewData = JSON.parse(readFileSync(resolve(generated, 'overview.json'), 'utf8'));
const catalog: Catalog = JSON.parse(readFileSync(resolve(generated, 'catalog.json'), 'utf8'));
type Face = 'display' | 'body' | 'mono';
const widths: Record<Face, Record<string, number>> = JSON.parse(readFileSync(resolve(fontDir, 'metrics.json'), 'utf8'));
const font = (face: Face) => readFileSync(resolve(fontDir, `${face}.woff`)).toString('base64');

const palettes: Record<Theme, Record<string, string>> = {
  dark: { bg: '#07090c', panel: '#0e1216', line: '#1e252c', ink: '#e4e7e9', muted: '#8f98a1', accent: '#ff7a2e', evidence: '#ff4d8d', applications: '#7c9cff', foundation: '#ff7a2e', platform: '#c9b27c', compute: '#a2a9b2' },
  light: { bg: '#f4f5f7', panel: '#ffffff', line: '#dde1e7', ink: '#13161b', muted: '#5a636e', accent: '#e2530a', evidence: '#d6336c', applications: '#4b5fc0', foundation: '#e2530a', platform: '#94793f', compute: '#5a626c' },
};
const flowColor = (p: Record<string, string>, k: Flow['kind']) => ({ data: p.applications, control: p.ink, evidence: p.evidence })[k];
const tiers: Tier[] = ['compute', 'platform', 'foundation', 'applications'];
const tierLabels: Record<Tier, string> = { applications: 'Your applications', foundation: 'doramcap', platform: 'Platform', compute: 'Compute' };
const SLAB_W = 17.5, SLAB_D = 7.4, TILE_H = .42;

// Color helpers.
const hex = (c: string) => [1, 3, 5].map(i => parseInt(c.slice(i, i + 2), 16));
const mix = (a: string, b: string, t: number) => '#' + hex(a).map((v, i) => Math.round(v + (hex(b)[i] - v) * t).toString(16).padStart(2, '0')).join('');
const f1 = (n: number) => Math.round(n * 10) / 10;
const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;');
const measure = (text: string, face: Face, size: number, spacing = 0) => [...text].reduce((w, c) => w + (widths[face][c] ?? .5) * size + spacing, 0);
function wrap(text: string, face: Face, size: number, max: number) {
  const lines: string[] = [];
  for (const word of text.split(' ')) {
    const last = lines.at(-1);
    if (last !== undefined && measure(`${last} ${word}`, face, size) <= max) lines[lines.length - 1] = `${last} ${word}`; else lines.push(word);
  }
  return lines;
}
// Deterministic random numbers keep regenerated files stable.
let seed = 7;
const rand = () => { seed = (seed * 1664525 + 1013904223) % 4294967296; return seed / 4294967296; };

// Stack nodes from stack.ts, placed like StoryScene's layout(): the foundation uses two rows.
interface Node { id: string; tier: Tier; x: number; z: number; w: number; d: number; }
const nodes: Node[] = [];
{
  const of = (t: Tier) => stackNodes(overview).filter(n => n.tier === t).map(n => n.id);
  const row = (tier: Tier, ids: string[], spacing: number, w: number, z = 0, d = 1.5) => ids.forEach((id, i) => nodes.push({ id, tier, x: (i - (ids.length - 1) / 2) * spacing, z, w, d }));
  row('applications', of('applications'), 3.9, 3.1);
  row('foundation', of('foundation').slice(0, 4), 4.05, 3.3, -1.55);
  row('foundation', of('foundation').slice(4), 4.05, 3.3, 1.55);
  const ints = overview.integrations;
  row('platform', [...ints.filter(i => i.group === 'execution'), ...ints.filter(i => i.group !== 'execution')].map(i => i.id), 2.7, 2.3);
  row('compute', of('compute'), 5.2, 4.3);
}

// Perspective camera from three.js spherical coordinates (radius, polar, azimuth) looking at a target.
interface Camera { pos: V3; right: V3; up: V3; fwd: V3; }
const sub = (a: V3, b: V3): V3 => ({ x: a.x - b.x, y: a.y - b.y, z: a.z - b.z });
const dot = (a: V3, b: V3) => a.x * b.x + a.y * b.y + a.z * b.z;
const cross = (a: V3, b: V3): V3 => ({ x: a.y * b.z - a.z * b.y, y: a.z * b.x - a.x * b.z, z: a.x * b.y - a.y * b.x });
const norm = (a: V3): V3 => { const l = Math.hypot(a.x, a.y, a.z); return { x: a.x / l, y: a.y / l, z: a.z / l }; };
function camera(radius: number, polar: number, azimuth: number, target: V3): Camera {
  const pos = { x: target.x + radius * Math.sin(polar) * Math.sin(azimuth), y: target.y + radius * Math.cos(polar), z: target.z + radius * Math.sin(polar) * Math.cos(azimuth) };
  const fwd = norm(sub(target, pos)), right = norm(cross(fwd, { x: 0, y: 1, z: 0 }));
  return { pos, fwd, right, up: cross(right, fwd) };
}
const project = (c: Camera, p: V3): P2 => { const v = sub(p, c.pos), z = dot(v, c.fwd); return { x: dot(v, c.right) / z, y: -dot(v, c.up) / z }; };

interface SceneOptions { theme: Theme; cam: Camera; gap: number; box: [number, number, number, number]; flows: Flow[]; states?: Record<string, 'on' | 'dim'>; intro: boolean; grid: boolean; }

function bezier(a: V3, b: V3, c: V3, d: V3, t: number): V3 {
  const u = 1 - t, k = [u * u * u, 3 * u * u * t, 3 * u * t * t, t * t * t];
  return { x: k[0] * a.x + k[1] * b.x + k[2] * c.x + k[3] * d.x, y: k[0] * a.y + k[1] * b.y + k[2] * c.y + k[3] * d.y, z: k[0] * a.z + k[1] * b.z + k[2] * c.z + k[3] * d.z };
}

/** Draws the platform stack and its flows into `box` (x, y, w, h). Returns SVG markup and the tier-tag anchors. */
function scene(o: SceneOptions) {
  const p = palettes[o.theme], dark = o.theme === 'dark';
  const slabY = (t: Tier) => tiers.indexOf(t) * o.gap;
  const blockPos = (n: Node): V3 => ({ x: n.x, y: slabY(n.tier) + TILE_H / 2 + .02, z: n.z });
  // Fit the projected slabs into the box.
  const corners = tiers.flatMap(t => [[-1, -1], [1, -1], [1, 1], [-1, 1]].map(([sx, sz]) => project(o.cam, { x: sx * SLAB_W / 2, y: slabY(t), z: sz * SLAB_D / 2 })));
  const minX = Math.min(...corners.map(c => c.x)), maxX = Math.max(...corners.map(c => c.x)), minY = Math.min(...corners.map(c => c.y)), maxY = Math.max(...corners.map(c => c.y));
  const [bx, by, bw, bh] = o.box, s = Math.min(bw / (maxX - minX), bh / (maxY - minY));
  const ox = bx + (bw - (maxX - minX) * s) / 2 - minX * s, oy = by + (bh - (maxY - minY) * s) / 2 - minY * s;
  const at = (v: V3) => { const q = project(o.cam, v); return { x: ox + q.x * s, y: oy + q.y * s }; };
  const poly = (pts: V3[]) => pts.map(at).map(q => `${f1(q.x)},${f1(q.y)}`).join(' ');
  const mid = { x: 0, y: 1.5 * o.gap, z: 0 }, unit = Math.hypot(at({ ...mid, x: 1 }).x - at(mid).x, at({ ...mid, x: 1 }).y - at(mid).y) * .1; // stroke and comet scale
  let out = '';

  // Floor grid, faded by a radial mask.
  if (o.grid) {
    const y = -2.4, r = 26, lines: string[] = [];
    for (let i = -r; i <= r; i += 2) lines.push(`M${poly([{ x: i, y, z: -r }]).replace(',', ' ')}L${poly([{ x: i, y, z: r }]).replace(',', ' ')}`, `M${poly([{ x: -r, y, z: i }]).replace(',', ' ')}L${poly([{ x: r, y, z: i }]).replace(',', ' ')}`);
    out += `<path d="${lines.join('')}" stroke="${p.line}" stroke-width="1" opacity="${dark ? .5 : .9}" mask="url(#fade)"/>`;
  }
  // Rising dust.
  if (dark) {
    let dust = '';
    for (let i = 0; i < 70; i++) {
      const q = at({ x: (rand() - .5) * 40, y: rand() * 16 - 3, z: (rand() - .5) * 30 });
      dust += `<circle class="dust" cx="${f1(q.x)}" cy="${f1(q.y)}" r="${f1(.6 + rand() * .9)}" style="animation-duration:${f1(9 + rand() * 9)}s;animation-delay:-${f1(rand() * 18)}s"/>`;
    }
    out += `<g fill="${p.muted}">${dust}</g>`;
  }

  // Slabs and blocks, bottom tier first; blocks far to near.
  const tags: Record<Tier, P2> = {} as Record<Tier, P2>;
  tiers.forEach((t, ti) => {
    const y = slabY(t), color = p[t];
    let g = `<polygon points="${poly([{ x: -SLAB_W / 2, y, z: -SLAB_D / 2 }, { x: SLAB_W / 2, y, z: -SLAB_D / 2 }, { x: SLAB_W / 2, y, z: SLAB_D / 2 }, { x: -SLAB_W / 2, y, z: SLAB_D / 2 }])}" fill="${color}" fill-opacity="${dark ? .05 : .07}" stroke="${color}" stroke-opacity="${dark ? .6 : .55}" stroke-width="1"/>`;
    const tierNodes = nodes.filter(n => n.tier === t).sort((a, b) => dot(sub(blockPos(b), o.cam.pos), o.cam.fwd) - dot(sub(blockPos(a), o.cam.pos), o.cam.fwd));
    for (const n of tierNodes) {
      const state = o.states?.[n.id] ?? 'rest', c = blockPos(n), hw = n.w / 2, hd = n.d / 2, hh = TILE_H / 2;
      const v = (sx: number, sy: number, sz: number): V3 => ({ x: c.x + sx * hw, y: c.y + sy * hh, z: c.z + sz * hd });
      const glow = { rest: dark ? .22 : .2, on: dark ? .42 : .5, dim: .04 }[state];
      const top = mix(p.panel, color, dark ? glow * 1.1 : glow * 1.5), side = mix(p.panel, color, dark ? glow * .55 : glow * 1.2);
      const shade = dark ? '#000000' : '#13161b';
      // Each face with its outward normal; draw the ones facing the camera.
      const faces: [V3[], V3, string][] = [
        [[v(-1, 1, -1), v(1, 1, -1), v(1, 1, 1), v(-1, 1, 1)], { x: 0, y: 1, z: 0 }, top],
        [[v(-1, -1, 1), v(1, -1, 1), v(1, 1, 1), v(-1, 1, 1)], { x: 0, y: 0, z: 1 }, mix(side, shade, dark ? .15 : .06)],
        [[v(-1, -1, -1), v(1, -1, -1), v(1, 1, -1), v(-1, 1, -1)], { x: 0, y: 0, z: -1 }, mix(side, shade, dark ? .15 : .06)],
        [[v(1, -1, -1), v(1, -1, 1), v(1, 1, 1), v(1, 1, -1)], { x: 1, y: 0, z: 0 }, mix(side, shade, dark ? .35 : .14)],
        [[v(-1, -1, -1), v(-1, -1, 1), v(-1, 1, 1), v(-1, 1, -1)], { x: -1, y: 0, z: 0 }, mix(side, shade, dark ? .35 : .14)],
      ];
      const edge = { rest: dark ? .4 : .5, on: .95, dim: .12 }[state];
      let block = '';
      for (const [pts, normal, fill] of faces) {
        const centre = pts.reduce((a, q) => ({ x: a.x + q.x / 4, y: a.y + q.y / 4, z: a.z + q.z / 4 }), { x: 0, y: 0, z: 0 });
        if (dot(normal, sub(o.cam.pos, centre)) <= 0) continue;
        block += `<polygon points="${poly(pts)}" fill="${fill}" stroke="${color}" stroke-opacity="${edge}"/>`;
      }
      // A soft pulse on top faces, staggered across the stack.
      if (dark && state !== 'dim') block += `<polygon class="pulse" points="${poly(faces[0][0])}" fill="${color}" style="animation-delay:-${f1((n.x + 9) * .5 + ti)}s"/>`;
      const delay = .35 + ti * .22 + (n.x + 9) / 18 * .3;
      g += `<g class="${o.intro ? 'pop' : ''}${state === 'dim' ? ' dim' : ''}" style="${o.intro ? `animation-delay:${f1(delay * 100) / 100}s` : ''}" stroke-linejoin="round">${block}</g>`;
    }
    tags[t] = at({ x: -SLAB_W / 2, y, z: SLAB_D / 2 + .5 });
    out += `<g class="${o.intro ? 'rise' : ''}" style="animation-delay:${f1(ti * .18 * 100) / 100}s"><g class="bob" style="animation-delay:-${ti * 1.3}s">${g}</g></g>`;
  });

  // Scan sheet sweeping up through the stack.
  const sheet = (y: number) => [{ x: -SLAB_W * .575, y, z: -SLAB_D * .625 }, { x: SLAB_W * .575, y, z: -SLAB_D * .625 }, { x: SLAB_W * .575, y, z: SLAB_D * .625 }, { x: -SLAB_W * .575, y, z: SLAB_D * .625 }];
  const low = at({ x: 0, y: -1.5, z: 0 }), high = at({ x: 0, y: 3 * o.gap + 1.5, z: 0 });
  out += `<g class="sweep" style="--dy:${f1(high.y - low.y)}px"><polygon points="${poly(sheet(-1.5))}" fill="url(#sheet)" stroke="${p.foundation}" stroke-opacity=".7"/></g>`;

  // Flows: a faint track plus three comets; reversed pairs run side by side.
  const anchor = (id: string, other: string): V3 => {
    const n = nodes.find(m => m.id === id);
    if (n) return blockPos(n);
    const b = blockPos(nodes.find(m => m.id === other)!);
    return { x: b.x, y: slabY('applications') + .05, z: b.z * .3 };
  };
  let tracks = '', comets = '';
  o.flows.forEach((f, i) => {
    const shift = pairOffset(f, o.flows) * .38;
    const a = anchor(f.from, f.to), b = anchor(f.to, f.from); a.x += shift; b.x += shift;
    let c1: V3, c2: V3;
    if (Math.abs(a.y - b.y) < .5) {
      const lift = .8 + Math.hypot(a.x - b.x, a.y - b.y, a.z - b.z) * .12;
      a.y += TILE_H / 2; b.y += TILE_H / 2;
      c1 = { ...a, y: a.y + lift }; c2 = { ...b, y: b.y + lift };
    } else { const mid = (a.y + b.y) / 2; c1 = { ...a, y: mid }; c2 = { ...b, y: mid }; }
    const d = 'M' + Array.from({ length: 41 }, (_, k) => at(bezier(a, c1, c2, b, k / 40))).map(q => `${f1(q.x)} ${f1(q.y)}`).join('L');
    const color = flowColor(p, f.kind), speed = `animation-duration:${f1(3.1 + (i % 3) * .2)}s;animation-delay:-${f1(rand() * 3)}s`;
    tracks += `<path d="${d}" stroke="${color}" stroke-opacity="${dark ? .3 : .45}"/>`;
    // pathLength=100 makes the dash pattern resolution-independent: a tail plus a bright head per comet.
    comets += `<path class="comet" d="${d}" pathLength="100" stroke="${color}" stroke-width="${f1(unit * (dark ? 3.4 : 2.2))}" stroke-dasharray="7 26.33" style="${speed}"/>`;
    comets += `<path class="comet head" d="${d}" pathLength="100" stroke="${dark ? mix(color, '#ffffff', .6) : color}" stroke-width="${f1(unit * (dark ? 5 : 3.2))}" stroke-dasharray="0.01 33.32" style="${speed}"/>`;
  });
  out += `<g fill="none" stroke-linecap="round"><g stroke-width="${f1(Math.max(1, unit * 1.2))}">${tracks}</g><g>${comets}</g></g>`;
  return { svg: out, tags, at };
}

const css = (theme: Theme, fonts: Face[]) => {
  const faces: Record<Face, [string, number]> = { display: ['Readme Display', 600], body: ['Readme Body', 400], mono: ['Readme Mono', 500] };
  return `<style>
${fonts.map(f => `@font-face{font-family:'${faces[f][0]}';font-weight:${faces[f][1]};src:url(data:font/woff;base64,${font(f)}) format('woff')}`).join('\n')}
.display{font-family:'Readme Display','IBM Plex Sans Condensed','Arial Narrow',sans-serif;font-weight:600}
.body{font-family:'Readme Body','IBM Plex Sans',system-ui,sans-serif}
.mono{font-family:'Readme Mono','IBM Plex Mono',ui-monospace,Menlo,monospace;font-weight:500;text-transform:uppercase}
.pop{transform-box:fill-box;transform-origin:center;animation:pop .7s cubic-bezier(.22,1,.36,1) both}
.dim{opacity:.32}
.rise{animation:rise 1.2s cubic-bezier(.22,1,.36,1) both}
.bob{animation:bob 9s ease-in-out infinite}
.pulse{opacity:0;animation:pulse 4.8s ease-in-out infinite}
.comet{animation-name:comet;animation-timing-function:linear;animation-iteration-count:infinite}
.head{stroke-dashoffset:-6.9;animation-name:comet-head}
.sweep{opacity:0;animation:sweep 7s 2.4s cubic-bezier(.45,0,.2,1) infinite}
.dust{opacity:.45;animation:dust linear infinite}
.beacon{transform-box:fill-box;transform-origin:center;animation:beacon 2.4s ease-out infinite}
@keyframes pop{from{transform:scale(.6)}}
@keyframes rise{from{transform:translateY(46px)}}
@keyframes bob{50%{transform:translateY(-3px)}}
@keyframes pulse{50%{opacity:.16}}
@keyframes comet{to{stroke-dashoffset:-33.33}}
@keyframes comet-head{to{stroke-dashoffset:-40.23}}
@keyframes sweep{0%{opacity:0;transform:translateY(0)}5%{opacity:${theme === 'dark' ? .55 : .4}}23%{opacity:0;transform:translateY(var(--dy))}100%{opacity:0;transform:translateY(var(--dy))}}
@keyframes dust{from{transform:translateY(0);opacity:0}15%{opacity:.45}85%{opacity:.45}to{transform:translateY(-60px);opacity:0}}
@keyframes beacon{from{opacity:.6;transform:scale(1)}70%,to{opacity:0;transform:scale(2.6)}}
@media (prefers-reduced-motion:reduce){*{animation:none!important}.sweep,.pulse{display:none}}
</style>`;
};

const defs = (theme: Theme, w: number, h: number, fadeAt: [number, number]) => {
  const p = palettes[theme];
  return `<defs>
<clipPath id="card"><rect width="${w}" height="${h}" rx="18"/></clipPath>
<radialGradient id="fade-g" cx="${fadeAt[0]}" cy="${fadeAt[1]}" r=".6"><stop offset=".2" stop-color="#fff"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></radialGradient>
<mask id="fade"><rect width="${w}" height="${h}" fill="url(#fade-g)"/></mask>
<linearGradient id="sheet" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${p.foundation}" stop-opacity=".04"/><stop offset=".5" stop-color="${p.foundation}" stop-opacity=".2"/><stop offset="1" stop-color="${p.foundation}" stop-opacity=".04"/></linearGradient>
<linearGradient id="vignette" x1="0" x2="1"><stop offset="0" stop-color="${p.bg}" stop-opacity=".94"/><stop offset=".34" stop-color="${p.bg}" stop-opacity=".6"/><stop offset=".56" stop-color="${p.bg}" stop-opacity="0"/></linearGradient>
<linearGradient id="floor" x1="0" y1="1" x2="0" y2="0"><stop offset="0" stop-color="${p.bg}"/><stop offset=".2" stop-color="${p.bg}" stop-opacity="0"/></linearGradient>
</defs>`;
};

const tagText = (theme: Theme, tags: Record<Tier, P2>, size: number) => tiers.map(t =>
  `<text class="mono" x="${f1(tags[t].x - 12)}" y="${f1(tags[t].y + size * .35)}" text-anchor="end" font-size="${size}" letter-spacing="${f1(size * .14)}" fill="${palettes[theme][t]}">${esc(tierLabels[t])}</text>`).join('');

// Every flow from every use case: the hero shows the whole platform in motion.
const heroFlows: Flow[] = [...new Map(overview.useCases.flatMap(u => u.flows).map(f => [`${f.from}>${f.to}`, f])).values()];

function hero(theme: Theme, stats: [number, string][]) {
  const W = 1280, H = 660, p = palettes[theme], dark = theme === 'dark';
  seed = 7;
  const cam = camera(43, 1.08, .74, { x: 0, y: 2.2, z: 0 });
  const st = scene({ theme, cam, gap: 1.25, box: [600, 70, 760, 540], flows: heroFlows, intro: true, grid: true });
  const words = overview.title.split(' ');
  const lines = ['Your runtime', 'foundation for', 'Physical AI.'];
  if (lines.join(' ') !== words.join(' ')) throw new Error(`Hero title changed: ${overview.title}`);
  const x = 72, size = 86;
  const accent = mix(p.accent, p.ink, dark ? .3 : .12);
  const title = lines.map((l, i) => `<text class="display" x="${x}" y="${222 + i * 82}" font-size="${size}" letter-spacing="-.86" fill="${i === 2 ? accent : p.ink}">${esc(l)}</text>`).join('');
  const summary = wrap(overview.summary, 'body', 20, 500).map((l, i) => `<text x="${x}" y="${440 + i * 31}">${esc(l)}</text>`).join('');
  const statsSvg = stats.map(([n, label], i) => `<g><text class="display" x="${x + i * 150}" y="${H - 82}" font-size="40" fill="${p.ink}">${n}</text><text class="mono" x="${x + i * 150}" y="${H - 56}" font-size="11.5" letter-spacing=".5" fill="${p.muted}" style="text-transform:none">${esc(label)}</text></g>`).join('');
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" role="img" aria-label="${esc(overview.title)} ${esc(overview.summary)}">
<title>${esc(overview.title)}</title>
${css(theme, ['display', 'body', 'mono'])}
${defs(theme, W, H, [.68, .55])}
<g clip-path="url(#card)">
<rect width="${W}" height="${H}" fill="${p.bg}"/>
${st.svg}
<rect width="${W}" height="${H}" fill="url(#vignette)"/>
<rect width="${W}" height="${H}" fill="url(#floor)"/>
<g><circle cx="${x + 4}" cy="126" r="4" fill="${p.accent}"/><circle class="beacon" cx="${x + 4}" cy="126" r="4" fill="${p.accent}"/>
<text class="mono" x="${x + 20}" y="130.5" font-size="12.5" letter-spacing="1.75" fill="${p.muted}">Source-available runtime · Rust · Dora · MCAP</text></g>
${title}
<g class="body" font-size="20" fill="${p.muted}">${summary}</g>
${statsSvg}
</g>
<rect x=".5" y=".5" width="${W - 1}" height="${H - 1}" rx="17.5" fill="none" stroke="${p.line}"/>
</svg>`;
}

function card(theme: Theme, index: number) {
  const W = 640, H = 400, p = palettes[theme], u = overview.useCases[index];
  seed = 11 + index;
  const cam = camera(47, 1.0, .72, { x: 0, y: 4.6, z: 0 });
  const lit = new Set<string>([...u.capabilities, ...u.integrations]);
  if (lit.has('orchestration')) lit.add('middleware');
  const states: Record<string, 'on' | 'dim'> = {};
  for (const n of nodes) if (n.tier === 'foundation' || n.tier === 'platform') states[n.id] = lit.has(n.id) ? 'on' : 'dim';
  const st = scene({ theme, cam, gap: 3.1, box: [120, 26, 520, 330], flows: u.flows, states, intro: false, grid: true });
  const kinds = [...new Set(u.flows.map(f => f.kind))];
  let lx = 32;
  const legend = kinds.map(k => {
    const label = flowLabels[k], c = flowColor(p, k);
    const g = `<rect x="${lx}" y="${H - 37}" width="20" height="2.5" rx="1" fill="${c}"/><text class="mono" x="${lx + 28}" y="${H - 31.5}" font-size="11.5" letter-spacing="1.2" fill="${p.muted}">${label}</text>`;
    lx += 28 + measure(label.toUpperCase(), 'mono', 11.5, 1.2) + 26;
    return g;
  }).join('');
  const pad = (n: number) => String(n).padStart(2, '0');
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" role="img" aria-label="${esc(u.title)}">
<title>${esc(u.title)}</title>
${css(theme, ['mono'])}
${defs(theme, W, H, [.6, .5])}
<g clip-path="url(#card)">
<rect width="${W}" height="${H}" fill="${p.bg}"/>
${st.svg}
<rect width="${W}" height="${H}" fill="url(#floor)"/>
${tagText(theme, st.tags, 9.5)}
<text class="mono" x="32" y="44" font-size="12" letter-spacing="1.7" fill="${p.muted}">Use case <tspan fill="${p.ink}">${pad(index + 1)}</tspan> / ${pad(overview.useCases.length)}</text>
${legend}
</g>
<rect x=".5" y=".5" width="${W - 1}" height="${H - 1}" rx="17.5" fill="none" stroke="${p.line}"/>
</svg>`;
}

// Same counts as the landing hero.
const stats: [number, string][] = [
  [catalog.components.filter(c => c.kind === 'crates').length, 'library crates'],
  [catalog.components.filter(c => c.kind === 'apps').length, 'runnable apps'],
  [overview.integrations.length, 'platform integrations'],
];
mkdirSync(out, { recursive: true });
for (const theme of ['dark', 'light'] as Theme[]) {
  writeFileSync(resolve(out, `hero-${theme}.svg`), hero(theme, stats));
  overview.useCases.forEach((u, i) => writeFileSync(resolve(out, `use-case-${u.id}-${theme}.svg`), card(theme, i)));
}
console.log(`Wrote README art for ${overview.useCases.length} use cases to ${out}.`);
