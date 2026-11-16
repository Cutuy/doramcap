import { wire } from '../../native.js';

export type Fact = { value: string; publicValue?: string; withheld?: boolean; unavailable?: boolean };
export type Facts = Map<string, Fact>;
const MAX_VALUE_CHARS = 512;
const bounded = (value: string) => value.length > MAX_VALUE_CHARS ? `${value.slice(0, MAX_VALUE_CHARS)}…` : value;

/** Compare exact values before withholding and truncation; absent evidence stays explicit. */
export function diffFacts(group: number, a: Facts, b: Facts) {
  const recorded = a.size > 0 && b.size > 0;
  return [...new Set([...a.keys(), ...b.keys()])].sort().map(name => {
    const left = a.get(name), right = b.get(name);
    const result = !recorded || left?.unavailable || right?.unavailable ? wire.ComparisonResult.COMPARISON_UNAVAILABLE
      : left?.value === right?.value ? wire.ComparisonResult.COMPARISON_SAME : wire.ComparisonResult.COMPARISON_CHANGED;
    const withheld = !!(left?.withheld || right?.withheld);
    return wire.ComparisonFact.create({ group, name, result,
      cells: [left, right].map(fact => ({
        values: [!fact || withheld ? '' : bounded(fact.publicValue ?? fact.value)], withheld,
        evidence: withheld ? '' : fact?.unavailable ? 'Unavailable' : '',
      })),
    });
  });
}
