import { wire } from '../native.js';
import { LIMITS } from '../generated/transport.js';

const MAX_COMPARISON_ROWS = 2000;
/** Retain nonmatching facts before matches, preserving module order within each set; report dropped rows. */
export function boundRows<Row extends { result?: number | null }>(rows: Row[]) {
  if (rows.length <= MAX_COMPARISON_ROWS) return { rows, truncated: false };
  const differs = (row: Row) => row.result !== wire.ComparisonResult.COMPARISON_RESULT_SAME;
  return { rows: [...rows.filter(differs), ...rows.filter(row => !differs(row))].slice(0, MAX_COMPARISON_ROWS), truncated: true };
}
/** Encode a complete comparison; oversized UTF-8 facts fail at the transport limit. */
export function comparisonResponse<T>(codec: { encode(value: T): { finish(): Uint8Array } }, value: T): Buffer {
  const bytes = Buffer.from(codec.encode(value).finish());
  if (bytes.length > LIMITS.frameBytes) throw Object.assign(new Error('Internal'), {
    detail: 'Comparison exceeds the supported ' + LIMITS.frameBytes + '-byte response limit.',
  });
  return bytes;
}
