import { componentHref, directDependencies, type Catalog } from './model';

/** Static direct declarations. Each link navigates normally; no viewport or layout engine. */
export default function Dependencies({ catalog, focus }: { catalog: Catalog; focus: string }) {
  const { packages, dependencies } = directDependencies(catalog, focus);
  if (!packages.length) return <p className="dependency-empty">No workspace dependencies.</p>;
  const normal = dependencies.filter(d => d.kind === 'normal');
  const extra = dependencies.filter(d => d.kind !== 'normal');
  function rows(edges: typeof dependencies) {
    const names = [...new Set(edges.map(e => e.target))].sort();
    return <ul className="direct-deps">{names.map(name => {
      const declarations = edges.filter(e => e.target === name);
      const pkg = catalog.packages.find(p => p.name === name)!;
      const owner = catalog.components.find(c => c.id === pkg.component)!;
      const notes = declarations.map(d => [packages.length > 1 ? d.source : '', d.kind !== 'normal' ? d.kind : '', d.optional ? 'optional' : '', d.condition ?? '', d.features.length ? `features: ${d.features.join(', ')}` : ''].filter(Boolean).join(' · ')).filter(Boolean);
      return <li key={name}><a href={componentHref(owner)}><code>{name}</code><span aria-hidden="true">→</span></a>{notes.length > 0 && <div className="dependency-notes">{notes.map((note, i) => <span key={i}>{note}</span>)}</div>}</li>;
    })}</ul>;
  }
  return <div className="direct-dependencies">{normal.length ? rows(normal) : <p className="dependency-empty">None.</p>}{extra.length > 0 && <details className="extra-deps"><summary>Build and development dependencies ({extra.length})</summary>{rows(extra)}</details>}</div>;
}
