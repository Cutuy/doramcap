import { wire } from '../../../native.js';
import type { CpuAnalysis, ServiceAnalysis } from './model.js';
const NS_PER_SECOND = 1_000_000_000n;
const MAX_FACT_CHARS = 512;
const key = (service: string, field: string) => 'service.' + service + '.' + field;
/** Preserve integer nanoseconds while expressing seconds in comparison values. */
const seconds = (ns: bigint) => String(ns / NS_PER_SECOND) + '.' + String(ns % NS_PER_SECOND).padStart(9, '0');
const bounded = (text: string) => text.length > MAX_FACT_CHARS ? text.slice(0, MAX_FACT_CHARS) + '…' : text;
const evidence = (value?: ServiceAnalysis) => !value || value.unavailable ? 'Unavailable: ' + (value?.unavailable ?? 'Service was not recorded')
  : (value.issues.length || value.elapsed < value.span ? 'Partial' : 'Measured') + ': ' + seconds(value.elapsed) + ' of ' + seconds(value.span) +
    ' recorded s' + (value.issues.length ? '; ' + value.issues.join(', ') : '');
/** Compare exact CPU measurements. Percent values are runtime_ns*100/elapsed_ns; missing intervals are unavailable. */
export function compareCpu(left: CpuAnalysis, right: CpuAnalysis) {
  const rows: typeof wire.ComparisonFact.prototype[] = [];
  const add = (name: string, values: (string | undefined)[], coverage: string[] = []) => {
    const unavailable = values.some(value => value === undefined);
    rows.push(wire.ComparisonFact.create({ group: wire.ComparisonGroup.COMPARISON_GROUP_OUTCOME, name,
      result: unavailable ? wire.ComparisonResult.COMPARISON_RESULT_UNAVAILABLE : values[0] === values[1] ? wire.ComparisonResult.COMPARISON_RESULT_SAME : wire.ComparisonResult.COMPARISON_RESULT_CHANGED,
      cells: values.map((value, side) => ({ values: [bounded(value ?? 'Unavailable')], evidence: bounded(coverage[side] ?? '') })),
    }));
  };
  if (left.unavailable || right.unavailable) rows.push(wire.ComparisonFact.create({ group: wire.ComparisonGroup.COMPARISON_GROUP_OUTCOME, name: 'cpu_capture',
    result: wire.ComparisonResult.COMPARISON_RESULT_UNAVAILABLE, cells: [left, right].map(value => ({ values: [bounded(value.unavailable ?? '')] })),
  }));
  for (const service of [...new Set([...left.services.keys(), ...right.services.keys()])].sort()) {
    const sides = [left.services.get(service), right.services.get(service)], coverage = sides.map(evidence);
    const measured = (format: (value: ServiceAnalysis) => string) => sides.map(value => value && !value.unavailable ? format(value) : undefined);
    add(key(service, 'cpu_coverage'), coverage, coverage);
    if (sides.some(value => !value || value.unavailable)) rows.at(-1)!.result = wire.ComparisonResult.COMPARISON_RESULT_UNAVAILABLE;
    add(key(service, 'cpu_observed_seconds'), measured(value => seconds(value.elapsed)), coverage);
    add(key(service, 'cpu_seconds'), measured(value => seconds(value.runtime)), coverage);
    // Rational values retain distinctions above Number precision; the presenter chooses readable percent text.
    add(key(service, 'cpu_mean_percent'), measured(value => value.elapsed > 0n ? String(value.runtime * 100n) + '/' + String(value.elapsed) : 'Unavailable'), coverage);
    const [a, b] = sides;
    if (a && b && !a.unavailable && !b.unavailable) rows.at(-1)!.result = a.elapsed <= 0n || b.elapsed <= 0n
      ? wire.ComparisonResult.COMPARISON_RESULT_UNAVAILABLE
      : a.runtime * b.elapsed === b.runtime * a.elapsed ? wire.ComparisonResult.COMPARISON_RESULT_SAME : wire.ComparisonResult.COMPARISON_RESULT_CHANGED;
  }
  return rows;
}
