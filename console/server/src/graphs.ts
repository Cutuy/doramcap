import type { doramcap } from './generated/console.js';
type App = doramcap.console.v1.ISystemAppDefinition;
type NativeGraph = { nodes: { id: string; [field: string]: unknown }[]; [field: string]: unknown };

/**
 * Merge Dora dataflow files into one run graph; Rust expands modules and validates the result.
 * A file lists nodes and wires outputs to inputs: `{nodes:[{id:'recorder',path:'dynamic',inputs:{data:'user/messages'}}]}`.
 */
export class Graphs {
  constructor(private native: {expandGraph(path:string):Promise<Buffer>}, private apps: App[], private base: string) {}
  /**
   * Return merged graph JSON from `dora.graph`, each selected app's file and `extra` paths, such as the live bridge node.
   * `profiles` maps app ID to launch profile ID; that profile's `graph` replaces the app's.
   */
  async build(requested?: string[], extra: string[] = [], profiles: Record<string, string> = {}): Promise<Buffer> {
    const ids = requested ?? this.apps.filter(app => app.defaultSelected).map(app => app.id!);
    const selected = new Set(ids);
    if (selected.size !== ids.length || ids.some(id => !this.apps.some(app => app.id === id))) throw new Error('InvalidInput');
    const paths = [this.base, ...this.apps.filter(app => selected.has(app.id!)).map(app => {
      const profile = app.launchProfiles?.find(profile => profile.id === profiles[app.id!]);
      return profile?.graph || app.graph || '';
    }), ...extra].filter(Boolean);
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
