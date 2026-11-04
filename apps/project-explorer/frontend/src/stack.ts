import type { Capability, Flow, Integration, OverviewData, UseCase } from './overview-model';

export type Tier = 'applications' | 'foundation' | 'platform' | 'compute';
export type NodeState = 'on' | 'dim' | 'selected';
/** One block in the platform stack. Selectable blocks carry the overview selection they link to. */
export interface StackNode {
  id: string; tier: Tier; title: string; note?: string;
  select?: { capability: string } | { integration: string };
}
/** Highlight state shared by the 3D and 2D views. Unlisted nodes render at rest. */
export interface StackView { states: Record<string, NodeState>; flows: Flow[]; }

const notes: Partial<Record<Integration['status'], string>> = { optional: 'optional', planned: 'planned', 'platform-dependent': 'platform-dependent' };

/** Build the stack from the overview manifest: workloads, foundation, middleware, platform and compute. */
export function stackNodes(overview: OverviewData): StackNode[] {
  return [
    ...overview.workloads.map((title, i) => ({ id: `workload-${i}`, tier: 'applications' as const, title })),
    ...overview.capabilities.map(c => ({ id: c.id, tier: 'foundation' as const, title: c.title, select: { capability: c.id } })),
    { id: 'middleware', tier: 'foundation', title: 'Existing middleware', note: 'Docker Compose', select: { capability: 'orchestration' } },
    ...overview.integrations.map(i => ({ id: i.id, tier: 'platform' as const, title: i.title, note: notes[i.status], select: { integration: i.id } })),
    ...overview.platforms.map((title, i) => ({ id: `compute-${i}`, tier: 'compute' as const, title })),
  ];
}

/** Light the selected use case, or narrow it to one capability or integration and its flows.
 * Without a use case, `allFlows` supplies the flows to narrow (the hero's combined set).
 */
export function stackView(overview: OverviewData, useCase?: UseCase, capability?: Capability, integration?: Integration, allFlows: Flow[] = []): StackView {
  const lit = new Set<string>();
  if (integration) [integration.id, ...overview.capabilities.filter(c => c.integrations.includes(integration.id)).map(c => c.id)].forEach(id => lit.add(id));
  else if (capability) [capability.id, ...capability.integrations].forEach(id => lit.add(id));
  else if (useCase) [...useCase.capabilities, ...useCase.integrations].forEach(id => lit.add(id));
  if (lit.has('orchestration')) lit.add('middleware');
  const focus = integration?.id ?? capability?.id;
  const states: Record<string, NodeState> = {};
  for (const id of [...overview.capabilities.map(c => c.id), 'middleware', ...overview.integrations.map(i => i.id)]) states[id] = id === focus ? 'selected' : lit.has(id) ? 'on' : 'dim';
  const flows = (useCase?.flows ?? allFlows).filter(f => !focus || f.from === focus || f.to === focus);
  return { states, flows };
}

/** Sideways offset for a flow whose reverse is also drawn, so the pair runs side by side. */
export function pairOffset(flow: Flow, flows: Flow[]): -1 | 0 | 1 {
  return flows.some(f => f.from === flow.to && f.to === flow.from) ? (flow.from < flow.to ? -1 : 1) : 0;
}

export const flowLabels: Record<Flow['kind'], string> = { data: 'Data', control: 'Control', evidence: 'Evidence' };
