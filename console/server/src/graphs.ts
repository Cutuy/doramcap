import type { doramcap } from './generated/console.js';
type App = doramcap.console.v1.ISystemAppDefinition;
type NativeNode = { id: string; outputs?: string[]; inputs?: Record<string, string | { source?: string }>; [field: string]: unknown };
type NativeGraph = { nodes: NativeNode[]; [field: string]: unknown };

/** A selected node subscribes to an output no selected graph provides. */
export class MissingSource extends Error {}
function requireSources(nodes: NativeNode[]) {
  for (const node of nodes) for (const input of Object.values(node.inputs ?? {})) {
    const source = typeof input === 'string' ? input : input.source ?? '';
    const [from, output] = source.split('/');
    if (from === 'dora' || nodes.some(item => item.id === from && item.outputs?.includes(output))) continue;
    throw new MissingSource(`${node.id} needs Dora output ${source}, which this selection does not provide. Deselect it or add the output to the project's flow.yml.`);
  }
}

/** Combine approved native graph fragments; Rust expands modules and validates the result. */
export class Graphs {
  constructor(private native: {expandGraph(path:string):Promise<Buffer>}, private apps: App[], private base: string) {}
  /** `extra` adds generated fragments, such as the live bridge node. */
  async build(requested?: string[], extra: string[] = []): Promise<Buffer> {
    const ids = requested ?? this.apps.filter(app => app.defaultSelected).map(app => app.id!);
    const selected = new Set(ids);
    if (selected.size !== ids.length || ids.some(id => !this.apps.some(app => app.id === id))) throw new Error('InvalidInput');
    const paths = [this.base, ...this.apps.filter(app => selected.has(app.id!)).map(app => app.graph ?? ''), ...extra].filter(Boolean);
    if (!paths.length) return Buffer.alloc(0);
    const merged: NativeGraph = {nodes:[]};
    const nodes = new Set<string>();
    for (const path of paths) {
      const bytes = await this.native.expandGraph(path);
      if (bytes.length > 2 * 1024 * 1024) throw new Error('InvalidInput');
      const graph = JSON.parse(bytes.toString()) as NativeGraph;
      if (!Array.isArray(graph.nodes)) throw new Error('InvalidInput');
      for (const node of graph.nodes) {
        if (!node.id || nodes.has(node.id)) throw new Error('InvalidInput');
        nodes.add(node.id); merged.nodes.push(node);
      }
      for (const [key,value] of Object.entries(graph)) {
        if (key === 'nodes') continue;
        if (key in merged && JSON.stringify(merged[key]) !== JSON.stringify(value)) throw new Error('InvalidInput');
        merged[key] = value;
      }
    }
    requireSources(merged.nodes);
    const bytes = Buffer.from(JSON.stringify(merged));
    if (bytes.length > 2 * 1024 * 1024) throw new Error('InvalidInput');
    return bytes;
  }
}
