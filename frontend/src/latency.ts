export interface ClockSample { serverMs: number; midpointMs: number; uncertaintyMs: number }
export function clockSample(serverNs: string, startedMs: number, endedMs: number): ClockSample {
  if (!/^\d+$/.test(serverNs) || endedMs < startedMs) throw new Error('Invalid clock sample');
  const ns = BigInt(serverNs);
  const serverMs = Number(ns / 1_000_000n) + Number(ns % 1_000_000n) / 1e6;
  if (ns <= 0n || !Number.isFinite(serverMs)) throw new Error('Invalid server clock');
  return { serverMs, midpointMs: (startedMs + endedMs) / 2, uncertaintyMs: (endedMs - startedMs) / 2 };
}
export function browserDelay(captureNs: string, receivedMs: number, clock: ClockSample): number | null {
  if (receivedMs - clock.midpointMs > 90_000) return null;
  const ns = BigInt(captureNs);
  if (ns <= 0n) return null;
  const capturedMs = Number(ns / 1_000_000n) + Number(ns % 1_000_000n) / 1e6;
  const delay = clock.serverMs + receivedMs - clock.midpointMs - capturedMs;
  return Number.isFinite(delay) && delay >= -clock.uncertaintyMs ? Math.max(0, delay) : null;
}
