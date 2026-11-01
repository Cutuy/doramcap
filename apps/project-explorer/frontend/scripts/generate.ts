import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync, mkdirSync, rmSync, copyFileSync, realpathSync } from 'node:fs';
import { dirname, resolve, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import { toString } from 'mdast-util-to-string';
import { visit } from 'unist-util-visit';
import { validateOverview } from './validate-overview';
import { relativePath, type Catalog, type Component } from '../src/model';

const frontend = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const root = resolve(frontend, '../../..');
const git = (...args: string[]) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--offline', '--no-deps', '--format-version', '1'], { cwd: root, encoding: 'utf8', maxBuffer: 20 * 1024 * 1024 }));
const config = JSON.parse(readFileSync(resolve(frontend, '../catalog.config.json'), 'utf8'));
const files = [...new Set(git('ls-files', '--cached', '--others', '--exclude-standard', '-z').split('\0').filter(Boolean))].filter(p => existsSync(resolve(root, p)));
const supported = files.filter(p => /^(README\.md$|(?:apps|console|libs|docs|scripts)\/)/.test(p) && !/(^|\/)(AGENTS\.md|node_modules|target|dist|\.generated)(\/|$)/.test(p));
const catalog: Catalog = { components: [], packages: [], dependencies: [], documents: {}, assets: [], files, repository: '', revision: git('rev-parse', 'HEAD'), warnings: [] };
const origin = git('remote', 'get-url', 'origin').replace(/\.git$/, '').replace(/^git@([^:]+):/, 'https://$1/');
if (!/^https:\/\/[^/@]+\//.test(origin)) throw new Error('A public HTTPS source repository URL is required.');
catalog.repository = origin;
const processor = unified().use(remarkParse).use(remarkGfm);
const summaries = new Map<string, string>();
const references: { document: string; url: string }[] = [];
for (const file of supported.filter(p => /\.md$/i.test(p))) {
  const text = readFileSync(resolve(root, file), 'utf8');
  const tree = processor.parse(text);
  const heading = tree.children.find(n => n.type === 'heading');
  const paragraph = tree.children.find(n => n.type === 'paragraph' && toString(n, { includeImageAlt: false, includeHtml: false }).trim());
  catalog.documents[file] = { title: heading ? toString(heading) : file, text };
  summaries.set(file, paragraph ? toString(paragraph, { includeImageAlt: false, includeHtml: false }).replace(/\s+/g, ' ').trim() : 'Read the component documentation.');
  visit(tree, node => { if ('url' in node && typeof node.url === 'string') references.push({ document: file, url: node.url }); });
}
function category(directory: string): string {
  const match = Object.keys(config.categories).sort((a, b) => b.length - a.length).find(p => directory === p || directory.startsWith(`${p}/`));
  return match ? config.categories[match] : 'Other';
}
function addComponent(kind: 'apps' | 'crates', name: string, directory: string): Component {
  const id = `${kind}/${name}`;
  const existing = catalog.components.find(c => c.id === id);
  if (existing) return existing;
  const readme = `${directory}/README.md`;
  if (!catalog.documents[readme]) throw new Error(`Missing component README: ${readme}`);
  const component: Component = { id, kind, name, directory, readme, category: category(directory), summary: summaries.get(readme)!, packages: [] };
  catalog.components.push(component); return component;
}
// Apps live under apps/<name>/; the top-level Console is also an app.
function appDirectory(directory: string): string | null {
  if (directory === 'console' || directory.startsWith('console/')) return 'console';
  return directory.startsWith('apps/') ? directory.split('/').slice(0, 2).join('/') : null;
}
const members = metadata.packages.filter((p: { id: string }) => metadata.workspace_members.includes(p.id));
for (const p of members) {
  const directory = relative(root, dirname(p.manifest_path)).split(sep).join('/');
  const app = appDirectory(directory);
  const component = app ? addComponent('apps', app.split('/').pop()!, app) : addComponent('crates', p.name, directory);
  component.packages.push(p.name);
  catalog.packages.push({ name: p.name, component: component.id, directory });
}
// Frontend-only apps participate in discovery without inventing Cargo dependencies.
for (const file of supported.filter(p => /^(apps\/[^/]+|console)\/README\.md$/.test(p))) { const directory = dirname(file); addComponent('apps', directory.split('/').pop()!, directory); }
const packagePaths = new Map(members.map((p: { name: string; manifest_path: string }) => [realpathSync(dirname(p.manifest_path)), p.name]));
for (const p of members) for (const dep of p.dependencies) {
  const target = dep.path && packagePaths.get(realpathSync(dep.path));
  if (typeof target !== 'string') continue;
  catalog.dependencies.push({ source: p.name, target, kind: dep.kind ?? 'normal', optional: dep.optional, condition: dep.target, features: dep.features });
}
const publicDir = resolve(frontend, 'public/content');
rmSync(publicDir, { recursive: true, force: true });
for (const file of supported.filter(p => /\.(svg|png|jpe?g|webp|gif)$/i.test(p))) {
  const path = realpathSync(resolve(root, file));
  if (!path.startsWith(root + sep)) throw new Error(`Asset escapes repository: ${file}`);
  const destination = resolve(publicDir, file); mkdirSync(dirname(destination), { recursive: true }); copyFileSync(path, destination); catalog.assets.push(file);
}
for (const ref of references) {
  if (/^(?:[a-z][a-z\d+.-]*:|\/\/|#)/i.test(ref.url)) continue;
  let path: string | null;
  try { path = relativePath(ref.document, decodeURIComponent(ref.url.split(/[?#]/)[0])); } catch { path = null; }
  if (!path || !files.some(f => f === path || f.startsWith(path + '/'))) catalog.warnings.push(`${ref.document}: ${ref.url}`);
}
catalog.components.sort((a, b) => a.name.localeCompare(b.name));
catalog.packages.sort((a, b) => a.name.localeCompare(b.name));
mkdirSync(resolve(frontend, 'src/.generated'), { recursive: true });
const overview = JSON.parse(readFileSync(resolve(frontend, '../overview.json'), 'utf8'));
validateOverview(overview, catalog);
const mappingDir = resolve(publicDir, 'apps/project-explorer');
mkdirSync(mappingDir, { recursive: true });
copyFileSync(resolve(frontend, '../overview.json'), resolve(mappingDir, 'overview.json'));
copyFileSync(resolve(frontend, '../overview.schema.json'), resolve(mappingDir, 'overview.schema.json'));
writeFileSync(resolve(frontend, 'src/.generated/overview.json'), JSON.stringify(overview));
writeFileSync(resolve(frontend, 'src/.generated/catalog.json'), JSON.stringify(catalog));
writeFileSync(resolve(frontend, 'src/.generated/link-report.json'), JSON.stringify(catalog.warnings, null, 2) + '\n');
console.log(`Generated ${catalog.components.length} components, ${catalog.dependencies.length} dependency declarations, ${Object.keys(catalog.documents).length} documents, ${catalog.assets.length} assets.`);
if (catalog.warnings.length) console.warn(`${catalog.warnings.length} existing documentation references need attention; see src/.generated/link-report.json.`);
