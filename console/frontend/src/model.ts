import type { Outcome, Snapshot } from './api';
export function toggleProfile(current: string[], name: string): string[] {
  return current.includes(name) ? current.filter(item => item !== name) : [...current, name];
}
export function createRequestKey(): string {
  return Array.from(crypto.getRandomValues(new Uint8Array(16)), byte => byte.toString(16).padStart(2, '0')).join('');
}
export function requestKey(existing: string, create: () => string): string { return existing || create(); }
export function acceptsSnapshot(expectedRun: string, snapshot: Snapshot): boolean {
  return !expectedRun || (snapshot.runId === expectedRun && ['STARTING', 'RUNNING', 'STOPPING', 'FINISHED', 'BLOCKED'].includes(snapshot.phase));
}
export function outcomeLabels(outcome: Outcome | null) {
  return {
    execution: outcome?.execution || 'Unknown',
    recording: outcome?.recordingFinalized == null ? 'Unknown' : outcome.recordingFinalized ? 'Saved' : 'Unavailable',
    cleanup: outcome?.boundaryClean == null ? 'Unknown' : outcome.boundaryClean ? 'Verified' : 'Unverified',
  };
}

/** Stop is a one-way transition; retries at the API remain idempotent. */
export const canStopRun = (snapshot: Pick<Snapshot, 'phase' | 'runId'> | null, pending = false) =>
  !pending && !!snapshot?.runId && ['STARTING', 'RUNNING'].includes(snapshot.phase);
