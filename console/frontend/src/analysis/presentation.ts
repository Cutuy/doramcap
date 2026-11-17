import type { HistoryComparison } from '../protocol';

export type ComparisonFact = HistoryComparison['rows'][number];
/** Shared renderer input; module presenters own names, formatting and visibility. */
export type ComparisonRowView = Pick<ComparisonFact, 'group' | 'name' | 'result'> & {
  label: string; hidden: boolean; alwaysVisible: boolean;
  cells: { values: string[]; evidence: string }[];
};
/** A shared privacy boundary for module presenters; the source facts remain unchanged. */
export function presentFact(fact: ComparisonFact, label: string, format: (value: string, others: string[]) => string, alwaysVisible = false): ComparisonRowView {
  const hidden = fact.cells.some(cell => cell.withheld), others = fact.cells.flatMap(cell => cell.values);
  return { group: fact.group, name: fact.name, result: fact.result, label, hidden, alwaysVisible,
    cells: fact.cells.map(cell => ({ values: hidden ? [] : cell.values.map(value => format(value, others)), evidence: hidden ? '' : cell.evidence })),
  };
}
