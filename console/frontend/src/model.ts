import { Phase, wire } from './api';
import type { HistoryEntry, Outcome, Snapshot } from './api';
export function toggleProfile(current: string[], name: string): string[] {
  return current.includes(name) ? current.filter(item => item !== name) : [...current, name];
}
/** One past run's result, matching its History state. */
export function pastRunLabel(snapshot: Pick<Snapshot, 'phase' | 'outcome'> | null): string {
  if (snapshot?.phase === Phase.BLOCKED) return 'Cleanup unverified';
  if (!snapshot?.outcome) return 'Interrupted run';
  if (snapshot.outcome.execution === 'Failed') return 'Failed run';
  return snapshot.outcome.execution === 'Cancelled' ? 'Cancelled run' : 'Completed run';
}
/** Track overlapping requests: only the latest begun and not cancelled one is current. */
export function latestRequest() {
  let latest = 0;
  return {
    begin() { const id = ++latest; return () => id === latest; },
    cancel() { latest++; },
  };
}
/** Run again starts a prepare, which the server refuses while another prepare or run is active. */
export function runAgainBlocked(snapshot: Pick<Snapshot, 'phase' | 'runId'> | null, preparing: boolean): boolean {
  if (preparing || snapshot?.phase === Phase.PREPARING) return true;
  return !!snapshot?.runId && [Phase.STARTING, Phase.RUNNING, Phase.STOPPING].includes(snapshot.phase);
}
export function createRequestKey(): string {
  return Array.from(crypto.getRandomValues(new Uint8Array(16)), byte => byte.toString(16).padStart(2, '0')).join('');
}
export function requestKey(existing: string, create: () => string): string { return existing || create(); }
export function acceptsSnapshot(expectedRun: string, snapshot: Pick<Snapshot, 'phase' | 'runId'>): boolean {
  return !expectedRun || (snapshot.runId === expectedRun && [Phase.STARTING, Phase.RUNNING, Phase.STOPPING, Phase.FINISHED, Phase.BLOCKED].includes(snapshot.phase));
}
export function outcomeLabels(outcome: Outcome | null) {
  return {
    execution: outcome?.execution || 'Unknown',
    recording: outcome?.recordingFinalized == null ? 'Unknown' : outcome.recordingFinalized ? 'Saved' : 'Unavailable',
    cleanup: outcome?.boundaryClean == null ? 'Unknown' : outcome.boundaryClean ? 'Verified' : 'Unverified',
  };
}

const { HistoryState: State } = wire;
const STATES: Partial<Record<number, { label: string; tone: 'active' | 'success' | 'attention' | 'failure' }>> = {
  [State.HISTORY_RUNNING]: { label: 'In progress', tone: 'active' },
  [State.HISTORY_COMPLETED]: { label: 'Completed', tone: 'success' },
  [State.HISTORY_FAILED]: { label: 'Failed', tone: 'failure' },
  [State.HISTORY_CANCELLED]: { label: 'Cancelled', tone: 'attention' },
  [State.HISTORY_BLOCKED]: { label: 'Cleanup unverified', tone: 'failure' },
  [State.HISTORY_INTERRUPTED]: { label: 'Interrupted', tone: 'attention' },
};
export const stateLabel = (state: number) => STATES[state]?.label ?? 'Unknown';
export const stateTone = (state: number) => STATES[state]?.tone ?? 'neutral';
/** The History view lives at `#/history` and `#/history/<run id>`; other hashes show the run workflow. */
export const historyHash = (runId = '') => `#/history${runId && `/${runId}`}`;
export function historyRoute(hash: string): { runId: string } | null {
  const match = /^#\/history(?:\/([a-f0-9]{32}))?$/.exec(hash);
  return match ? { runId: match[1] ?? '' } : null;
}
export const timeLabel = (ms: unknown) => new Date(Number(ms)).toLocaleString();
export function durationLabel(entry: Pick<HistoryEntry, 'state' | 'startedAtMs' | 'finishedAtMs'>): string {
  if (entry.state === State.HISTORY_RUNNING) return 'In progress';
  if (entry.finishedAtMs == null) return 'Unknown';
  const seconds = Math.max(0, Number(entry.finishedAtMs) - Number(entry.startedAtMs)) / 1000;
  if (seconds < 60) return `${seconds.toFixed(1)} s`;
  const whole = Math.round(seconds), hours = Math.floor(whole / 3600), minutes = Math.floor(whole % 3600 / 60);
  return hours ? `${hours} h ${minutes} min` : `${minutes} min ${whole % 60} s`;
}
/** Total size of recording files on disk; the report does not count. */
export function entryRecordingBytes(entry: Pick<HistoryEntry, 'artifacts'>): number | null {
  const sizes = entry.artifacts.filter(artifact => artifact.id !== 'report' && artifact.sizeBytes != null);
  return sizes.length ? sizes.reduce((total, artifact) => total + Number(artifact.sizeBytes), 0) : null;
}

/** Stop is a one-way transition; retries at the API remain idempotent. */
export const canStopRun = (snapshot: Pick<Snapshot, 'phase' | 'runId'> | null, pending = false) =>
  !pending && !!snapshot?.runId && [Phase.STARTING, Phase.RUNNING].includes(snapshot.phase);
