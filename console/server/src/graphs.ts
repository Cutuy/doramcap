import type { doramcap } from './generated/console.cjs';
type App = doramcap.console.v1.ISystemAppDefinition;
type NativeGraph = { nodes: { id: string; [field: string]: unknown }[]; [field: string]: unknown };

/** Combine approved native graph fragments; Rust expands modules and validates the result. */
export class Graphs {
  constructor(private native: {expandGraph(path:string):Promise<Buffer>}, private apps: App[], private base: string) {}
  async build(requested?: string[]): Promise<Buffer> {
    const ids = requested ?? this.apps.filter(app => app.defaultSelected).map(app => app.id!);
    const selected = new Set(ids);
    if (selected.size !== ids.length || ids.some(id => !this.apps.some(app => app.id === id))) throw new Error('InvalidInput');
    const paths = [this.base, ...this.apps.filter(app => selected.has(app.id!)).map(app => app.graph ?? '')].filter(Boolean);
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
    const bytes = Buffer.from(JSON.stringify(merged));
    if (bytes.length > 2 * 1024 * 1024) throw new Error('InvalidInput');
    return bytes;
  }
}
