export type Kind = 'apps' | 'crates';
export interface Component {
  id: string; name: string; kind: Kind; category: string; directory: string;
  readme: string; summary: string; packages: string[];
}
export interface Package { name: string; component: string; directory: string; }
export interface Dependency {
  source: string; target: string; kind: 'normal' | 'build' | 'dev'; optional: boolean;
  condition: string | null; features: string[];
}
export interface Document { title: string; text: string; }
export interface Catalog {
  components: Component[]; packages: Package[]; dependencies: Dependency[];
  documents: Record<string, Document>; assets: string[]; files: string[];
  repository: string; revision: string; warnings: string[];
}
const acronyms: Record<string, string> = { cpu: 'CPU', mcap: 'MCAP', cuda: 'CUDA', shm: 'SHM', bpf: 'BPF', pbtxt: 'PBTXT', dora: 'Dora' };
export function displayName(name: string): string {
  return name.split('-').map(word => acronyms[word] ?? word.charAt(0).toUpperCase() + word.slice(1)).join(' ');
}
export function componentHref(component: Component): string {
  return `#/${component.id}`;
}
export interface Route { path: string; params: URLSearchParams; }
export function parseRoute(hash: string): Route {
  const [path, query = ''] = (hash.replace(/^#/, '') || '/overview').split('?');
  return { path, params: new URLSearchParams(query) };
}
export function routeHref(path: string, params: URLSearchParams): string {
  return `#${path}${params.size ? `?${params}` : ''}`;
}
/** Resolve a repository-relative reference; return null if it escapes the root. */
export function relativePath(document: string, reference: string): string | null {
  const parts = reference.startsWith('/') ? [] : document.split('/').slice(0, -1);
  for (const part of reference.split('/')) {
    if (!part || part === '.') continue;
    if (part === '..') { if (!parts.length) return null; parts.pop(); }
    else parts.push(part);
  }
  return parts.join('/');
}
export function assetHref(path: string, base: string): string {
  return `${base}content/${path.split('/').map(encodeURIComponent).join('/')}`;
}
/** Keep documentation local, route component directories, and link source at the build revision. */
export function resolveLink(url: string, document: string, catalog: Catalog, base = './'): string {
  if (/^(https?:|mailto:|tel:)/i.test(url) || url.startsWith('//')) return url;
  if (/^[a-z][a-z\d+.-]*:/i.test(url)) return '';
  const hash = url.indexOf('#');
  const anchor = hash < 0 ? '' : url.slice(hash + 1);
  const raw = (hash < 0 ? url : url.slice(0, hash)).split('?')[0];
  let decoded: string;
  try { decoded = decodeURIComponent(raw); } catch { return ''; }
  const path = raw ? relativePath(document, decoded) : document;
  if (path === null) return '';
  const component = catalog.components.find(c => c.directory === path || c.readme === path);
  const doc = catalog.documents[path] ? path : catalog.documents[`${path}/README.md`] ? `${path}/README.md` : null;
  const query = new URLSearchParams();
  if (anchor) { try { query.set('heading', decodeURIComponent(anchor)); } catch { query.set('heading', anchor); } }
  if (component) return routeHref(`/${component.id}`, query);
  if (doc) { query.set('path', doc); return routeHref('/docs', query); }
  if (catalog.assets.includes(path)) return assetHref(path, base) + (anchor ? `#${anchor}` : '');
  const isFile = catalog.files.includes(path);
  const source = `${catalog.repository}/${isFile ? 'blob' : 'tree'}/${catalog.revision}/${path.split('/').map(encodeURIComponent).join('/')}`;
  return source + (anchor ? `#${anchor}` : '');
}
/** Outgoing one-hop declarations for a package or all packages owned by an app.
 * Internal edges between the app's own packages are kept visible and annotated.
 */
export function directDependencies(catalog: Catalog, focus: string) {
  const component = catalog.components.find(c => c.id === focus || c.name === focus);
  const packages = catalog.packages.filter(p => component ? component.packages.includes(p.name) : p.name === focus);
  const names = new Set(packages.map(p => p.name));
  return { packages, dependencies: catalog.dependencies.filter(d => names.has(d.source)) };
}

/** Show transitive dependencies and, for a focus, transitive dependents.
 * Traverse each direction separately to exclude unrelated sibling branches.
 * A visited set terminates traversal even when dev/build declarations contain cycles.
 */
export function dependencyChain(catalog: Catalog, focus: string, kinds: string[]) {
  const roots = focus
    ? directDependencies(catalog, focus).packages.map(p => p.name)
    : catalog.packages.map(p => p.name);
  const eligible = catalog.dependencies.filter(d => kinds.includes(d.kind));
  const walk = (reverse: boolean) => {
    const visited = new Set(roots);
    const pending = [...roots];
    while (pending.length) {
      const current = pending.pop()!;
      for (const edge of eligible) {
        const source = reverse ? edge.target : edge.source;
        const target = reverse ? edge.source : edge.target;
        if (source === current && !visited.has(target)) {
          visited.add(target); pending.push(target);
        }
      }
    }
    return visited;
  };
  const included = walk(false);
  if (focus) for (const name of walk(true)) included.add(name);
  return { roots, packages: catalog.packages.filter(p => included.has(p.name)), dependencies: eligible.filter(d => included.has(d.source) && included.has(d.target)) };
}
