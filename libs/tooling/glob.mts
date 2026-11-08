import { Minimatch } from 'minimatch';

/** Compile case-sensitive minimatch patterns, including dotfiles; match when any pattern accepts the name. */
export function globMatcher(patterns: readonly string[]): (name: string) => boolean {
  const matchers = patterns.map(pattern => new Minimatch(pattern, { dot: true }));
  return name => matchers.some(matcher => matcher.match(name));
}
