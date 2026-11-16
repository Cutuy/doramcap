import { wire } from '../../../native.js';
import type { CpuAnalysis, ServiceAnalysis } from './model.js';
const NS_PER_SECOND = 1_000_000_000n;
const MAX_PRESENTATION_CHARS = 512;
const key = (service: string, field: string) => 'service.' + service + '.' + field;
/** Preserve integer nanoseconds while expressing seconds in comparison values. */
const seconds = (ns: bigint) => String(ns / NS_PER_SECOND) + '.' + String(ns % NS_PER_SECOND).padStart(9, '0');
const bounded = (text: string) => text.length > MAX_PRESENTATION_CHARS ? text.slice(0, MAX_PRESENTATION_CHARS) + '…' : text;
const evidence = (value?: ServiceAnalysis) => !value || value.unavailable ? 'Unavailable: ' + (value?.unavailable ?? 'Service was not recorded')
  : (value.issues.length || value.elapsed < value.span ? 'Partial' : 'Measured') + ': ' + seconds(value.elapsed) + ' of ' + seconds(value.span) +
    ' recorded s' + (value.issues.length ? '; ' + value.issues.join(', ') : '');
/** Exact measurement comparison and source-specific labels, units and evidence stay in this module. */
export function compareCpu(left: CpuAnalysis, right: CpuAnalysis) {
  const rows: typeof wire.ComparisonRow.prototype[] = [];
  const add = (name: string, label: string, values: (string | undefined)[], display: string[], coverage: string[], alwaysVisible = false) => {
    const unavailable = values.some(value => value === undefined);
    rows.push(wire.ComparisonRow.create({ group: wire.ComparisonGroup.COMPARISON_OUTCOME, name, label: bounded(label), alwaysVisible,
      result: unavailable ? wire.ComparisonResult.COMPARISON_UNAVAILABLE : values[0] === values[1] ? wire.ComparisonResult.COMPARISON_SAME : wire.ComparisonResult.COMPARISON_CHANGED,
      cells: values.map((value, side) => ({ values: [bounded(value ?? 'Unavailable')], displayValues: bounded(display[side] ?? '') === bounded(value ?? 'Unavailable') ? [] : [bounded(display[side] ?? '')], evidence: bounded(coverage[side] ?? '') })),
    }));
  };
  for (const service of [...new Set([...left.services.keys(), ...right.services.keys()])].sort()) {
    const sides = [left.services.get(service), right.services.get(service)], coverage = sides.map(evidence);
    const measured = (format: (value: ServiceAnalysis) => string) => sides.map(value => value && !value.unavailable ? format(value) : undefined);
    const runtime = measured(value => seconds(value.runtime));
    const utilization = measured(value => String(Number(value.runtime) / Number(value.elapsed) * 100));
    const observed = measured(value => seconds(value.elapsed));
    const withUnit = (values: (string | undefined)[], unit: string) => values.map(value => value === undefined ? 'Unavailable' : value + unit);
    add(key(service, 'cpu_seconds'), service + ' · Measured CPU time', runtime, withUnit(runtime, ' s'), coverage);
    add(key(service, 'cpu_mean_percent'), service + ' · Mean CPU · core %', utilization, withUnit(utilization, '%'), coverage);
    const [a, b] = sides;
    if (a && b && !a.unavailable && !b.unavailable) {
      const equal = a.runtime * b.elapsed === b.runtime * a.elapsed;
      const row = rows.at(-1)!;
      row.result = equal ? wire.ComparisonResult.COMPARISON_SAME : wire.ComparisonResult.COMPARISON_CHANGED;
      if (!equal && utilization[0] === utilization[1]) row.cells.forEach((cell, side) => {
        const measurement = sides[side]!;
        cell.displayValues = [String(measurement.runtime * 100n) + '/' + String(measurement.elapsed) + ' %'];
      });
    }
    add(key(service, 'cpu_observed_seconds'), service + ' · Measured duration', observed, withUnit(observed, ' s'), coverage, true);
    add(key(service, 'cpu_coverage'), service + ' · CPU coverage', sides.map(value => value && !value.unavailable ? evidence(value) : undefined), coverage, [], true);
  }
  if (left.unavailable || right.unavailable) {
    const values = [left.unavailable, right.unavailable];
    add('cpu_capture', 'CPU recording', [undefined, undefined], values.map(value => value ? 'Unavailable: ' + value : 'Recorded'), [], true);
  }
  return rows;
}
