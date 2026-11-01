import type { doramcap } from './generated/console';
type Snapshot = doramcap.telemetry.v1.ISnapshot;
const integer = (value: unknown) => BigInt(String(value ?? 0));

/** Rank existing leaf cgroup counters individually; do not sum ranked groups. */
export function topCpuGroups(snapshots: Snapshot[]) {
  const [before, after] = snapshots.slice(-2);
  if (!before || !after || integer(after.sequence) <= integer(before.sequence) || integer(after.capacityOverflow) !== 0n || integer(after.droppedBatches) !== 0n) return [];
  const elapsed = integer(after.monotonicEndNs) - integer(before.monotonicEndNs);
  if (elapsed <= 0n) return [];
  const counters = new Map((before.records ?? []).filter(r => (Number(r.flags) & 1) !== 0 && (Number(r.flags) & (16 | 64 | 128)) === 0).map(r => [`${r.cgroupId}:${r.cpu}`, integer(r.runtimeNs)]));
  const parents = new Set((after.records ?? []).map(row => String(row.parentId ?? 0)));
  const groups = new Map<string, { id: string; delta: bigint; valid: boolean }>();
  for (const row of after.records ?? []) {
    if (row.cpu === 0xffffffff || (Number(row.flags) & (16 | 128)) !== 0) continue;
    const id = String(row.cgroupId ?? 0);
    if (parents.has(id)) continue;
    const group = groups.get(id) ?? {id, delta: 0n, valid: true};
    const old = counters.get(`${id}:${row.cpu}`);
    const current = integer(row.runtimeNs);
    if ((Number(row.flags) & 1) === 0 || (Number(row.flags) & 64) !== 0 || old === undefined || current < old) group.valid = false;
    else group.delta += current - old;
    groups.set(id, group);
  }
  return [...groups.values()].filter(group => group.valid).map(group => ({id: group.id, percent: Number(group.delta) / Number(elapsed) * 100})).sort((a,b) => b.percent - a.percent || a.id.localeCompare(b.id)).slice(0, 5);
}
