import type { Integration } from './overview-model';
import { pairOffset, type StackNode, type StackView, type Tier } from './stack';

const W = 1120, H = 520;
const bands: Record<Tier, { y: number; h: number; label: string }> = {
  applications: { y: 0, h: 84, label: 'Your applications' },
  foundation: { y: 100, h: 164, label: 'doramcap' },
  platform: { y: 280, h: 136, label: 'Platform' },
  compute: { y: 432, h: 80, label: 'Compute' },
};
interface Box { cx: number; cy: number; w: number; h: number; }
// Gaps between foundation blocks; flows between platform and applications climb through them.
const foundationGaps = [159, 293, 427, 561, 695, 829];

/** Place every node in schematic coordinates. Platform integrations split into Runtime and Data groups. */
function layout(nodes: StackNode[], integrations: Integration[]): Map<string, Box> {
  const boxes = new Map<string, Box>();
  const tier = (t: Tier) => nodes.filter(n => n.tier === t);
  tier('applications').forEach((n, i, all) => boxes.set(n.id, { cx: W / 2 + (i - (all.length - 1) / 2) * 240, cy: 50, w: 200, h: 34 }));
  tier('foundation').filter(n => n.id !== 'middleware').forEach((n, i) => boxes.set(n.id, { cx: 92 + i * 134, cy: 196, w: 122, h: 56 }));
  boxes.set('middleware', { cx: 1035, cy: 196, w: 138, h: 112 });
  for (const [group, start] of [['execution', 120], ['communication', 660]] as const)
    integrations.filter(i => (i.group ?? 'communication') === group).forEach((i, k) => boxes.set(i.id, { cx: start + k * 170, cy: 370, w: 150, h: 50 }));
  tier('compute').forEach((n, i, all) => boxes.set(n.id, { cx: W / 2 + (i - (all.length - 1) / 2) * 260, cy: 478, w: 220, h: 34 }));
  return boxes;
}

function path(from: string, to: string, boxes: Map<string, Box>, shift: number): string {
  const apps = bands.applications.y + bands.applications.h;
  const a = boxes.get(from), b = boxes.get(to);
  // `applications` is the whole tier: meet it straight above the other endpoint.
  if (!a || !b) {
    const other = (a ?? b)!, up = !b ? 1 : -1, x = other.cx + shift;
    const top = other.cy - other.h / 2;
    if (other.cy > bands.platform.y) {
      const gap = foundationGaps.reduce((best, g) => Math.abs(g - x) < Math.abs(best - x) ? g : best) + shift;
      const below = bands.foundation.y + bands.foundation.h;
      const d = `M${x},${top} C${x},${top - 40} ${gap},${below + 30} ${gap},${below} L${gap},${apps + 4}`;
      return up < 0 ? `M${gap},${apps} L${gap},${below} C${gap},${below + 30} ${x},${top - 40} ${x},${top - 4}` : d;
    }
    return up > 0 ? `M${x},${top} L${x},${apps + 4}` : `M${x},${apps} L${x},${top - 4}`;
  }
  const ax = a.cx + shift, bx = b.cx + shift;
  if (Math.abs(a.cy - b.cy) < 1) {
    const y = a.cy + a.h / 2, dip = y + 34;
    return `M${ax},${y} C${ax},${dip} ${bx},${dip} ${bx},${y + 4}`;
  }
  const down = a.cy < b.cy;
  const y0 = down ? a.cy + a.h / 2 : a.cy - a.h / 2, y1 = down ? b.cy - b.h / 2 - 4 : b.cy + b.h / 2 + 4, my = (y0 + y1) / 2;
  return `M${ax},${y0} C${ax},${my} ${bx},${my} ${bx},${y1}`;
}

/** Flat schematic of the platform stack with the selected use case's flows. */
export default function StackSchematic({ nodes, integrations, view, hrefFor }: {
  nodes: StackNode[]; integrations: Integration[]; view: StackView; hrefFor: (node: StackNode) => string | undefined;
}) {
  const boxes = layout(nodes, integrations);
  return <svg className="schematic" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Platform stack schematic">
    <defs>{(['data', 'control', 'evidence'] as const).map(k => <marker key={k} id={`arrow-${k}`} viewBox="0 0 10 10" refX="5" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,1 L9,5 L0,9 z" className={`arrow-${k}`} /></marker>)}</defs>
    {(Object.entries(bands) as [Tier, typeof bands[Tier]][]).map(([tier, b]) => <g key={tier} className={`band band-${tier}`}>
      <rect x="1" y={b.y + 1} width={W - 2} height={b.h - 2} rx="8" /><text x="18" y={b.y + 22}>{b.label}</text>
    </g>)}
    <text className="group-label" x="45" y="330">Runtime</text><text className="group-label" x="585" y="330">Data</text>
    {view.flows.map(f => { const shift = pairOffset(f, view.flows) * 9; return <g key={`${f.from}-${f.to}`} className={`flow flow-${f.kind}`}>
      <path className="flow-track" d={path(f.from, f.to, boxes, shift)} />
      <path className="flow-dash" d={path(f.from, f.to, boxes, shift)} markerEnd={`url(#arrow-${f.kind})`} />
    </g>; })}
    {nodes.map(n => {
      const box = boxes.get(n.id)!, href = hrefFor(n), state = view.states[n.id] ?? 'rest';
      const body = <>
        <rect className="node-box" x={box.cx - box.w / 2} y={box.cy - box.h / 2} width={box.w} height={box.h} rx="6" />
        <text className="node-title" x={box.cx} y={box.cy + (n.note ? -3 : 4)}>{n.title}</text>
        {n.note && <text className="node-note" x={box.cx} y={box.cy + 13}>{n.note}</text>}
      </>;
      const className = `node node-${n.tier} ${n.id === 'middleware' ? 'node-middleware ' : ''}${state}`;
      return href ? <a key={n.id} href={href} className={className} aria-label={n.title}>{body}</a> : <g key={n.id} className={className}>{body}</g>;
    })}
  </svg>;
}
