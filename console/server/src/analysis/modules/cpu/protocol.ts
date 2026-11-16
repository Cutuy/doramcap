import { doramcap } from '../../../generated/console.js';
import type { CpuAnalysis } from './model.js';
const protocol = doramcap.console.analysis.cpu.v1;
/** Encode exact arithmetic as the module-owned per-run protobuf, shared with future Rust adapters. */
export function encodeAnalysis(value: CpuAnalysis): Uint8Array {
  return protocol.Analysis.encode(protocol.Analysis.fromObject({ unavailable: value.unavailable,
    services: Object.fromEntries([...value.services].map(([name, item]) => [name, {
      runtimeNs: item.runtime.toString(), elapsedNs: item.elapsed.toString(), spanNs: item.span.toString(),
      issues: item.issues, unavailable: item.unavailable,
    }])),
  })).finish();
}
/** Decode native values into the module's private bigint arithmetic model without precision loss. */
export function decodeAnalysis(bytes: Uint8Array): CpuAnalysis {
  const value = protocol.Analysis.decode(bytes);
  return { unavailable: value.unavailable || undefined, services: new Map(Object.entries(value.services).map(([name, item]) => [name, {
    runtime: BigInt(String(item.runtimeNs ?? 0)), elapsed: BigInt(String(item.elapsedNs ?? 0)), span: BigInt(String(item.spanNs ?? 0)),
    issues: item.issues ?? [], unavailable: item.unavailable || undefined,
  }])) };
}
