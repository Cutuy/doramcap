export interface Mapping {
  id: string; title: string; summary: string; components: string[]; evidence: string[];
}
export interface Capability extends Mapping {
  category: string; status: 'building-blocks' | 'planned'; limits: string; integrations: string[];
}
export interface Integration extends Mapping { url: string; group?: 'execution' | 'communication'; status: 'integrated' | 'optional' | 'planned' | 'platform-dependent'; }
/** A directed edge in a use case; endpoints are its capabilities, integrations or `applications`. */
export interface Flow { from: string; to: string; kind: 'data' | 'control' | 'evidence'; }
export interface UseCase extends Mapping {
  outcome: string; status: 'illustrative' | 'validated'; capabilities: string[]; integrations: string[]; flows: Flow[];
}
export interface OverviewData {
  version: 1; title: string; summary: string; workloads: string[]; platforms: string[]; embodiments: string[];
  capabilities: Capability[]; integrations: Integration[]; useCases: UseCase[];
}
