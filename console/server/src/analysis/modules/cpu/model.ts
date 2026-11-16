/** Exact per-run arithmetic stays private to this module. */
export type ServiceAnalysis = { runtime: bigint; elapsed: bigint; span: bigint; issues: string[]; unavailable?: string };
export type CpuAnalysis = { services: Map<string, ServiceAnalysis>; unavailable?: string };
