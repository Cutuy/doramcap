import { wire } from '../native.js';
import { LIMITS } from '../generated/transport.js';

type Row = typeof wire.ComparisonRow.prototype;
const MAX_COMPARISON_ROWS = 2000;
/** Retain producer-marked context, differences, then matching facts within the row budget. */
export function boundRows(rows: Row[]) {
  if (rows.length <= MAX_COMPARISON_ROWS) return { rows, truncated: false };
  const context = rows.filter(row => row.alwaysVisible), facts = rows.filter(row => !row.alwaysVisible);
  const differs = (row: Row) => row.result !== wire.ComparisonResult.COMPARISON_SAME;
  return { rows: [...context, ...facts.filter(differs), ...facts.filter(row => !differs(row))].slice(0, MAX_COMPARISON_ROWS), truncated: true };
}
/** Encode a complete comparison; oversized UTF-8 presentation fails at the transport limit. */
export function comparisonResponse<T>(codec: { encode(value: T): { finish(): Uint8Array } }, value: T): Buffer {
  const bytes = Buffer.from(codec.encode(value).finish());
  if (bytes.length > LIMITS.frameBytes) throw Object.assign(new Error('Internal'), {
    detail: 'Comparison exceeds the supported ' + LIMITS.frameBytes + '-byte response limit.',
  });
  return bytes;
}
