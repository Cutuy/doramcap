import type { CpuInfo } from 'node:os';

/** Return host utilization in core percent; warm-up, resets and CPU changes yield null. */
export function cpuUsage(before: CpuInfo[], after: CpuInfo[]) {
  const sum = (rows: CpuInfo[]) => rows.reduce((result, { times }) => ({
    busy: result.busy + times.user + times.nice + times.sys + times.irq,
    idle: result.idle + times.idle,
  }), { busy: 0, idle: 0 });
  const old = sum(before), current = sum(after);
  const busy = current.busy - old.busy, idle = current.idle - old.idle;
  return {
    cpuCores: after.length,
    cpuPercent: before.length === after.length && after.length > 0 && busy >= 0 && idle >= 0 && busy + idle > 0
      ? busy / (busy + idle) * after.length * 100
      : null,
  };
}
