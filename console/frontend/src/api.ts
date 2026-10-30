import { readLive } from './live';
import type { doramcap } from './generated/console';
export interface Service { name: string; imageId: string }
export interface SystemApp { id: string; label: string; selected: boolean }
export interface Review { preparedId: string; runId: string; availableProfiles: string[]; selectedProfiles: string[]; services: Service[]; warnings: string[]; systemApps: SystemApp[] }
export interface StepDescriptor { id: string; displayName: string; retention: string; kind: string }
export interface Step { descriptor?: StepDescriptor | null; session: string; executor: string; invocation: string; step: string; state: string; durationNs: string; elapsedNs?: string; parent?: string | null; dependencies?: string[]; sequence?: string; stepVersion?: number; error?: string | null }
export interface Outcome { execution: string; writersStopped?: boolean | null; recordingFinalized?: boolean | null; boundaryClean?: boolean | null; eventStreamHealthy?: boolean | null }
export interface RunMetric { elapsedNs: string; recordingBytes?: string | null; writeBytesPerSecond?: number | null; cpuPercent?: number | null; cpuCores: number }
export interface Snapshot { phase: string; runId: string; steps: Step[]; sequence: string; lostEvents: string; truncatedSteps: string; overflowSteps: string; progressIncomplete: boolean; outcome: Outcome | null; cpuAvailable: boolean; reportAvailable: boolean; monitorReady: boolean; projectDirectory?: string; profiles?: string[]; metrics?: RunMetric[]; activityAvailable?: boolean; metricsIncomplete?: boolean; cpuSnapshots?: doramcap.telemetry.v1.ISnapshot[]; cpuGroupNames?: { id: string; name: string }[]; systemApps?: { ids: string[] } | null }
export class ApiError extends Error { constructor(public code: string) { super(code); } }
async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, { ...init, headers: { ...(init?.body ? { 'content-type': 'application/json' } : {}), ...init?.headers }, credentials: 'same-origin' });
  const value = await response.json();
  if (!response.ok) throw new ApiError(value?.error?.code ?? 'Internal');
  return value as T;
}
const post = <T>(path: string, value: unknown) => request<T>(path, { method: 'POST', body: JSON.stringify(value) });
export const api = {
  prepare: (directory: string, profiles: string[], systemApps?: string[]) => post<Review>('/api/prepare', { directory, profiles, ...(systemApps === undefined ? {} : { systemApps }) }),
  start: (preparedId: string, requestKey: string) => post<{ runId: string }>('/api/start', { preparedId, requestKey }),
  stop: (runId: string) => post<{ accepted: boolean }>('/api/stop', { runId }),
  current: () => request<Snapshot>('/api/current'),
  live: readLive,
};

export const cpuRecordingEnabled = (snapshot: Pick<Snapshot, 'systemApps'> | null) => snapshot?.systemApps == null || snapshot.systemApps.ids.includes('cpu-monitor');
