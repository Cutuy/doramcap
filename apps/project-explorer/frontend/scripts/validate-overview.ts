import Ajv from 'ajv';
import schema from '../../overview.schema.json';
import type { Catalog } from '../src/model';
import type { OverviewData } from '../src/overview-model';
const validate = new Ajv({ allErrors: true }).compile(schema);

/** Validate the versioned editorial manifest against its schema and generated repository inventory.
 * Throws an actionable error for malformed data, duplicate IDs, dangling references,
 * or use-case components outside the declared capabilities. It never modifies inputs.
 */
export function validateOverview(value: unknown, catalog: Catalog): asserts value is OverviewData {
  if (!validate(value)) throw new Error(`Overview schema: ${JSON.stringify(validate.errors)}`);
  const data = value as unknown as OverviewData;
  const components = new Set(catalog.components.map(c => c.id));
  const capabilities = new Map(data.capabilities.map(c => [c.id, c]));
  const integrations = new Set(data.integrations.map(i => i.id));
  for (const [name, items] of [['capabilities', data.capabilities], ['integrations', data.integrations], ['useCases', data.useCases]] as const) {
    const ids = new Set<string>();
    for (const item of items) {
      if (ids.has(item.id)) throw new Error(`Duplicate ${name} ID: ${item.id}`);
      ids.add(item.id);
      for (const id of item.components) if (!components.has(id)) throw new Error(`${name}/${item.id}: unknown component ${id}`);
      for (const path of item.evidence) if (!catalog.documents[path]) throw new Error(`${name}/${item.id}: unknown evidence document ${path}`);
      if ('integrations' in item) for (const id of item.integrations) if (!integrations.has(id)) throw new Error(`${name}/${item.id}: unknown integration ${id}`);
    }
  }
  for (const item of data.useCases) {
    for (const id of item.capabilities) if (!capabilities.has(id)) throw new Error(`${item.id}: unknown capability ${id}`);
    const members = new Set(item.capabilities.flatMap(id => capabilities.get(id)!.components));
    for (const id of item.components) if (!members.has(id)) throw new Error(`${item.id}: component ${id} is outside its capabilities`);
    const partners = new Set(item.capabilities.flatMap(id => capabilities.get(id)!.integrations));
    for (const id of item.integrations) if (!partners.has(id)) throw new Error(`${item.id}: integration ${id} is outside its capabilities`);
  }
}
