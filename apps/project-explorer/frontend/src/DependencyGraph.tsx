import { useEffect, useMemo, useState } from 'react';
import { ReactFlow, Background, Controls, MarkerType, Position, type Node, type Edge } from '@xyflow/react';
import ELK from 'elkjs/lib/elk.bundled.js';
import '@xyflow/react/dist/style.css';
import { componentHref, dependencyChain, type Catalog } from './model';

const elk = new ELK();
export default function DependencyGraph({ catalog, focus, category, build, dev }: { catalog: Catalog; focus: string; category: string; build: boolean; dev: boolean }) {
  const graph = useMemo(() => dependencyChain(catalog, focus, category, ['normal', ...(build ? ['build'] : []), ...(dev ? ['dev'] : [])]), [catalog, focus, category, build, dev]);
  const [nodes, setNodes] = useState<Node[]>([]);
  const [edges, setEdges] = useState<Edge[]>([]);
  const [error, setError] = useState('');
  const [selected, setSelected] = useState('');
  const [ready, setReady] = useState(false);
  useEffect(() => {
    let cancelled = false;
    setReady(false); setSelected(''); setError('');
    // Combine duplicate normal/build/dev declarations into one visual edge.
    const grouped = new Map<string, typeof graph.dependencies>();
    for (const dependency of graph.dependencies) {
      const id = `${dependency.source}->${dependency.target}`;
      grouped.set(id, [...(grouped.get(id) ?? []), dependency]);
    }
    const edgeList: Edge[] = [...grouped].map(([id, deps]) => ({
      id, source: deps[0].source, target: deps[0].target,
      markerEnd: { type: MarkerType.ArrowClosed },
      label: deps.some(d => d.optional || d.condition) ? 'conditional' : deps.map(d => d.kind).filter(k => k !== 'normal').join(', '),
      style: { strokeDasharray: deps.some(d => d.optional || d.condition) ? '5 4' : undefined },
      ariaLabel: deps.map(d => `${d.source} depends on ${d.target}: ${d.kind}${d.optional ? ', optional' : ''}${d.condition ? `, ${d.condition}` : ''}`).join('; '),
    }));
    elk.layout({ id: 'workspace', layoutOptions: { 'elk.algorithm': 'layered', 'elk.direction': 'DOWN', 'elk.spacing.nodeNode': '30', 'elk.layered.spacing.nodeNodeBetweenLayers': '65' },
      children: graph.packages.map(p => ({ id: p.name, width: 224, height: 70 })),
      edges: edgeList.map(e => ({ id: e.id, sources: [e.source], targets: [e.target] })),
    }).then(layout => {
      if (cancelled) return;
      setNodes((layout.children ?? []).map(n => {
        const pkg = graph.packages.find(p => p.name === n.id)!;
        const component = catalog.components.find(c => c.id === pkg.component)!;
        return { id: n.id, position: { x: n.x ?? 0, y: n.y ?? 0 }, sourcePosition: Position.Bottom, targetPosition: Position.Top,
          data: { label: <><span className="node-kind">{component.kind === 'apps' ? 'App package' : 'Crate'}</span><span>{pkg.name}</span></> },
          style: { width: 224, height: 70 }, className: component.id === focus || pkg.name === focus || component.name === focus ? 'focus-node' : '',
        };
      }));
      setEdges(edgeList); setReady(true);
    }).catch(e => { if (!cancelled) { setError(String(e)); setReady(true); } });
    return () => { cancelled = true; };
  }, [graph, catalog, focus]);
  const pkg = catalog.packages.find(p => p.name === selected);
  const component = catalog.components.find(c => c.id === pkg?.component);
  if (!graph.packages.length) return <p className="empty">No workspace dependencies.</p>;
  return <>
    <div className="graph-canvas original-graph" aria-label="Interactive package dependency graph">
      {error ? <p role="alert">Graph layout failed. Please reload to try again.</p> : !ready ? <p className="loading" role="status">Arranging dependencies…</p> :
        <ReactFlow key={JSON.stringify({ focus, category, build, dev })} nodes={nodes} edges={edges} fitView fitViewOptions={{ padding: 0.2, maxZoom: 1 }} minZoom={0.12} maxZoom={1.8} nodesDraggable={false} nodesConnectable={false} elementsSelectable colorMode="system" onNodeClick={(_, node) => setSelected(node.id)} onSelectionChange={({ nodes: selection }) => { if (selection[0]) setSelected(selection[0].id); }}>
          <Background gap={22} size={1} /><Controls showInteractive={false} />
        </ReactFlow>}
    </div>
    {component && <div className="node-inspector" aria-live="polite"><div><strong>{selected}</strong><p>{component.summary}</p></div><a href={componentHref(component)}>Open {component.kind === 'apps' ? 'app' : 'crate'} →</a></div>}
  </>;
}
