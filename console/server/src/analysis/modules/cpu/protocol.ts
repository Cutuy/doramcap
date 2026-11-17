import { doramcap } from '../../../generated/console.js';
import type { CpuAnalysis } from './model.js';
const protocol = doramcap.console.analysis.cpu.v1;
const MAX_ANALYSIS_NS = (1n << 64n) - 1n;
const nanoseconds = (value: bigint): string => {
  if (value < 0n || value > MAX_ANALYSIS_NS) throw new RangeError('CPU analysis exceeds uint64 nanoseconds');
  return value.toString();
};
/** Encode exact arithmetic as the module-owned per-run protobuf, shared with future Rust adapters. */
export function encodeAnalysis(value: CpuAnalysis): Uint8Array {
  return protocol.Analysis.encode(protocol.Analysis.fromObject({ unavailable: value.unavailable,
    services: [...value.services].map(([service, item]) => ({ service,
      runtimeNs: nanoseconds(item.runtime), elapsedNs: nanoseconds(item.elapsed), spanNs: nanoseconds(item.span),
      issues: item.issues, unavailable: item.unavailable,
    })),
  })).finish();
}
/** Decode native values into the module's private bigint arithmetic model without precision loss. */
export function decodeAnalysis(bytes: Uint8Array): CpuAnalysis {
  const value = protocol.Analysis.decode(bytes);
  return { unavailable: value.unavailable || undefined, services: new Map(value.services.map(item => [item.service ?? '', {
    runtime: BigInt(String(item.runtimeNs ?? 0)), elapsed: BigInt(String(item.elapsedNs ?? 0)), span: BigInt(String(item.spanNs ?? 0)),
    issues: item.issues ?? [], unavailable: item.unavailable || undefined,
  }])) };
}
