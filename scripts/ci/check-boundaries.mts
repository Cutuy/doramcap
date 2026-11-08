#!/usr/bin/env node
import { Command, cli, isMain, canonicalPath, globMatcher, capture } from '../../libs/tooling/tooling.mts';
// Check forbidden Cargo dependency paths using workspace path dependencies.
import { readFileSync } from 'node:fs';
import { dirname } from 'node:path';

type Dependency = { name: string; kind?: string | null; path?: string };
export type Metadata = {
  workspace_members: string[];
  packages: { id: string; name: string; manifest_path: string; dependencies: Dependency[] }[];
};
type Rule = { from: string; forbid: string[]; kinds?: string[] };

/** Return unique forbidden dependency paths; reject malformed policies and ambiguous workspace names. */
export function violations(metadata: Metadata, config: unknown): string[] {
  if (!config || typeof config !== 'object' || Object.keys(config).join() !== 'rules'
      || !('rules' in config) || !Array.isArray(config.rules) || !config.rules.length) {
    throw new Error('config requires a nonempty rules list and no other keys');
  }
  const members = new Set(metadata.workspace_members);
  const packages = metadata.packages.filter(pkg => members.has(pkg.id));
  const byName = new Map(packages.map(pkg => [pkg.name, pkg]));
  if (byName.size !== packages.length) throw new Error('ambiguous workspace package names');
  const errors = new Set<string>();
  for (const input of config.rules) {
    if (!input || typeof input !== 'object' || Array.isArray(input)
        || Object.keys(input).some(key => !['from', 'forbid', 'kinds'].includes(key))) {
      throw new Error('unknown rule fields');
    }
    const { from: source, forbid: patterns, kinds = ['normal', 'build'] } = input as Rule;
    if (typeof source !== 'string' || !byName.has(source)) throw new Error(`unknown source package: ${source}`);
    if (!Array.isArray(patterns) || !patterns.length || patterns.some(p => typeof p !== 'string' || !p)) {
      throw new Error('forbid must be a nonempty list of patterns');
    }
    if (!Array.isArray(kinds) || !kinds.length || kinds.some(k => !['normal', 'build', 'dev'].includes(k))) {
      throw new Error('invalid dependency kinds');
    }
    const forbidden = globMatcher(patterns);
    const queue: [string, string[]][] = [[source, [source]]];
    const visited = new Set([source]);
    for (let index = 0; index < queue.length; index++) {
      const [name, path] = queue[index];
      for (const dependency of byName.get(name)!.dependencies) {
        if (!kinds.includes(dependency.kind || 'normal')) continue;
        const target = dependency.name;
        const nextPath = [...path, target];
        if (forbidden(target)) errors.add(nextPath.join(' -> '));
        // External packages may share a workspace member's name.
        const pkg = byName.get(target);
        const local = pkg && dependency.path && canonicalPath(dependency.path) === dirname(canonicalPath(pkg.manifest_path));
        if (local && !visited.has(target)) {
          visited.add(target);
          queue.push([target, nextPath]);
        }
      }
    }
  }
  return [...errors].sort();
}

if (isMain(import.meta.url)) await cli(new Command()
  .description('Check forbidden Cargo dependency paths.')
  .option('--repo <directory>', 'source checkout', process.cwd())
  .requiredOption('--rules <file>', 'dependency rules JSON')
  .option('--metadata <file>', 'existing Cargo metadata JSON')
  .action((values: { repo: string; rules: string; metadata?: string }) => {
    const config: unknown = JSON.parse(readFileSync(values.rules, 'utf8'));
    const metadata: Metadata = JSON.parse(values.metadata ? readFileSync(values.metadata, 'utf8') : capture(
      'cargo', ['metadata', '--no-deps', '--offline', '--format-version', '1'], { cwd: values.repo },
    ).stdout);
    const found = violations(metadata, config);
    for (const path of found) console.log(`forbidden dependency: ${path}`);
    if (!found.length) console.log('Dependency boundaries passed (workspace paths and direct dependencies).');
    process.exitCode = found.length ? 1 : 0;
  }), 2);
