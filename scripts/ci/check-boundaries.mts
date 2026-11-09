#!/usr/bin/env node
import assert from 'node:assert/strict';
import { readFileSync, realpathSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Command, CommanderError } from 'commander';
import { execaSync } from 'execa';
import { minimatch } from 'minimatch';
export type Metadata = { workspace_members: string[]; packages: {
  id: string; name: string; manifest_path: string; dependencies: { name: string; kind?: string | null; path?: string }[];
}[] };
function canonical(path: string): string {
  try { return realpathSync(path); } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    return join(canonical(dirname(resolve(path))), basename(path));
  }
}
/** Return unique forbidden paths, following workspace dependencies and validating every rule. */
export function violations(metadata: Metadata, config: any): string[] {
  assert(config && Object.keys(config).join() === 'rules' && Array.isArray(config.rules) && config.rules.length, 'config requires a nonempty rules list and no other keys');
  const packages = metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id));
  const byName = new Map(packages.map(pkg => [pkg.name, pkg]));
  assert.equal(byName.size, packages.length, 'ambiguous workspace package names');
  const errors = new Set<string>();
  for (const rule of config.rules) {
    assert(rule && !Array.isArray(rule) && Object.keys(rule).every(key => ['from', 'forbid', 'kinds'].includes(key)), 'unknown rule fields');
    const { from: source, forbid, kinds = ['normal', 'build'] } = rule;
    assert(typeof source === 'string' && byName.has(source), `unknown source package: ${source}`);
    assert(Array.isArray(forbid) && forbid.length && forbid.every(p => typeof p === 'string' && p), 'forbid must be a nonempty list of patterns');
    assert(Array.isArray(kinds) && kinds.length && kinds.every(k => ['normal', 'build', 'dev'].includes(k)), 'invalid dependency kinds');
    const queue: string[][] = [[source]], visited = new Set([source]);
    for (const path of queue) for (const dep of byName.get(path.at(-1)!)!.dependencies) {
      if (!kinds.includes(dep.kind || 'normal')) continue;
      const next = [...path, dep.name], pkg = byName.get(dep.name);
      if (forbid.some(pattern => minimatch(dep.name, pattern, { dot: true }))) errors.add(next.join(' -> '));
      // External packages may share a workspace member's name.
      if (pkg && dep.path && canonical(dep.path) === dirname(canonical(pkg.manifest_path)) && !visited.has(dep.name)) {
        visited.add(dep.name);
        queue.push(next);
      }
    }
  }
  return [...errors].sort();
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const args = new Command().exitOverride().option('--repo <directory>', 'source checkout', process.cwd())
    .requiredOption('--rules <file>', 'dependency rules JSON').option('--metadata <file>', 'Cargo metadata JSON').parse().opts();
    const metadata = args.metadata ? readFileSync(args.metadata, 'utf8') : execaSync('cargo',
      ['metadata', '--no-deps', '--offline', '--format-version', '1'], { cwd: args.repo, maxBuffer: Infinity }).stdout;
    const found = violations(JSON.parse(metadata), JSON.parse(readFileSync(args.rules, 'utf8')));
    for (const path of found) console.log(`forbidden dependency: ${path}`);
    if (!found.length) console.log('Dependency boundaries passed (workspace paths and direct dependencies).');
    process.exitCode = found.length ? 1 : 0;
  } catch (error) {
    if (!(error instanceof CommanderError)) console.error((error as Error).message);
    process.exitCode = error instanceof CommanderError && error.exitCode === 0 ? 0 : 2;
  }
}
