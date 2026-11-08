#!/usr/bin/env node
// Check forbidden Cargo dependency paths using workspace path dependencies.
import { execFileSync } from 'node:child_process';
import { readFileSync, realpathSync } from 'node:fs';
import { basename, dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

type Dependency = { name: string; kind?: string | null; path?: string };
export type Metadata = {
  workspace_members: string[];
  packages: { id: string; name: string; manifest_path: string; dependencies: Dependency[] }[];
};
type Rule = { from: string; forbid: string[]; kinds?: string[] };

/** Match case-sensitive shell patterns, including Python fnmatch's literal backslashes and brackets. */
export function matchesPattern(name: string, pattern: string): boolean {
  const literal = (character: string) => `\\u{${character.codePointAt(0)!.toString(16)}}`;
  const characters = [...pattern];
  let expression = '';
  for (let i = 0; i < characters.length; i++) {
    const character = characters[i];
    if (character === '*') { expression += '.*'; continue; }
    if (character === '?') { expression += '.'; continue; }
    if (character === '[') {
      let start = i + 1;
      const negate = characters[start] === '!';
      if (negate) start++;
      let end = start + (characters[start] === ']' ? 1 : 0);
      while (end < characters.length && characters[end] !== ']') end++;
      if (end < characters.length) {
        let ranges = '';
        for (let j = start; j < end; j++) {
          if (j + 2 < end && characters[j + 1] === '-') {
            if (characters[j].codePointAt(0)! <= characters[j + 2].codePointAt(0)!) {
              ranges += `${literal(characters[j])}-${literal(characters[j + 2])}`;
            }
            j += 2;
          } else ranges += literal(characters[j]);
        }
        expression += ranges ? `[${negate ? '^' : ''}${ranges}]` : negate ? '.' : '(?!)';
        i = end;
        continue;
      }
    }
    expression += literal(character);
  }
  return new RegExp(`^(?:${expression})$(?!.)`, 'su').test(name);
}

function canonical(path: string): string {
  try { return realpathSync(path); }
  catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    const absolute = resolve(path);
    return resolve(canonical(dirname(absolute)), basename(absolute));
  }
}

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
    const queue: [string, string[]][] = [[source, [source]]];
    const visited = new Set([source]);
    for (let index = 0; index < queue.length; index++) {
      const [name, path] = queue[index];
      for (const dependency of byName.get(name)!.dependencies) {
        if (!kinds.includes(dependency.kind || 'normal')) continue;
        const target = dependency.name;
        const nextPath = [...path, target];
        if (patterns.some(pattern => matchesPattern(target, pattern))) errors.add(nextPath.join(' -> '));
        // External packages may share a workspace member's name.
        const pkg = byName.get(target);
        const local = pkg && dependency.path && canonical(dependency.path) === dirname(canonical(pkg.manifest_path));
        if (local && !visited.has(target)) {
          visited.add(target);
          queue.push([target, nextPath]);
        }
      }
    }
  }
  return [...errors].sort();
}

function main(): number {
  try {
    const { values } = parseArgs({ options: {
      repo: { type: 'string', default: process.cwd() }, rules: { type: 'string' },
      metadata: { type: 'string' }, help: { type: 'boolean', short: 'h' },
    } });
    if (values.help) { console.log('Usage: node check-boundaries.mts --rules FILE [--repo DIR] [--metadata FILE]'); return 0; }
    if (!values.rules) throw new Error('--rules is required');
    const config: unknown = JSON.parse(readFileSync(values.rules, 'utf8'));
    const metadata: Metadata = JSON.parse(values.metadata ? readFileSync(values.metadata, 'utf8') : execFileSync(
      'cargo', ['metadata', '--no-deps', '--offline', '--format-version', '1'],
      { cwd: values.repo, encoding: 'utf8', maxBuffer: Infinity },
    ));
    const found = violations(metadata, config);
    for (const path of found) console.log(`forbidden dependency: ${path}`);
    if (!found.length) console.log('Dependency boundaries passed (workspace paths and direct dependencies).');
    return found.length ? 1 : 0;
  } catch (error) { console.error(`boundary check failed: ${(error as Error).message}`); return 2; }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) process.exitCode = main();
