import { MEDIA_TYPE, LIMITS, artifactPath, runFilePath, historyRunPath, isErrorCode, type ErrorCode } from './generated/transport';
import type { doramcap } from './generated/console';
export { PREPARING_RUN } from './generated/transport';
import { readLive } from './live';
import type { Frame } from './streams';

export * from './protocol';
import { wire, routes, type HistoryDetail, type Snapshot } from './protocol';
export class ApiError extends Error {
  constructor(public code: ErrorCode, public detail = '', public check = '') {
    super(code);
  }
}

async function request<T>(
  endpoint: { path: string; method: string; status: number; response: { decode(bytes: Uint8Array): T } },
  body: Uint8Array | undefined,
  fetcher: typeof fetch,
  signal?: AbortSignal,
): Promise<T> {
  const response = await fetcher(endpoint.path, {
    method: endpoint.method,
    body: body ? new Blob([new Uint8Array(body)]) : undefined,
    headers: { accept: MEDIA_TYPE, ...(body ? { 'content-type': MEDIA_TYPE } : {}) },
    credentials: 'same-origin',
    signal,
  });
  if (response.headers.get('content-type')?.split(';')[0].trim() !== MEDIA_TYPE)
    throw new Error('Invalid API response type');
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (bytes.length > LIMITS.frameBytes) throw new Error('API response exceeds limit');
  if (!response.ok) {
    const error = routes.error.decode(bytes);
    throw new ApiError(isErrorCode(error.code) ? error.code : 'Internal', error.message, error.check);
  }
  if (response.status !== endpoint.status) throw new Error('Invalid API response status');
  return endpoint.response.decode(bytes);
}
/** Encode the complete Review input, preserving omitted versus explicit system-app selection. */
export function encodePrepareRequest(profiles: string[], systemApps?: string[], appProfiles: Record<string, string> = {}, baselineRunId = '', writePolicy?: doramcap.filesystem.v1.WritePolicy): Uint8Array {
  return routes.prepare.request.encode(wire.PrepareRequest.create({
    profiles,
    baselineRunId,
    writePolicy,
    ...(systemApps === undefined ? {} : { systemApps: wire.SystemAppSelection.create({ ids: systemApps, profiles: appProfiles }) }),
  })).finish();
}

/** Create the browser client with an explicit transport for HTTP integration tests. */
export function createClient(fetcher: typeof fetch = fetch) {
  return {
    prepare: (profiles: string[], systemApps?: string[], appProfiles: Record<string, string> = {}, baselineRunId = '', writePolicy?: doramcap.filesystem.v1.WritePolicy) =>
      request(routes.prepare, encodePrepareRequest(profiles, systemApps, appProfiles, baselineRunId, writePolicy), fetcher),
    start: (preparedId: string, requestKey: string) =>
      request(
        routes.start,
        routes.start.request.encode(wire.StartRequest.create({ preparedId, requestKey })).finish(),
        fetcher,
      ),
    stop: (runId: string) =>
      request(routes.stop, routes.stop.request.encode(wire.StopRequest.create({ runId })).finish(), fetcher),
    current: (): Promise<Snapshot> => request(routes.current, undefined, fetcher),
    history: (query: { state?: number; cursor?: string; limit?: number }) =>
      request(routes.history, routes.history.request.encode(wire.HistoryQuery.create(query)).finish(), fetcher),
    historyRun: (runId: string): Promise<HistoryDetail> => request({ ...routes.historyRun, path: historyRunPath(runId) }, undefined, fetcher),
    historyCompare: (a: string, b: string, signal?: AbortSignal) =>
      request(
        routes.historyCompare,
        routes.historyCompare.request.encode(wire.HistoryComparisonRequest.create({ sides: [wire.ComparisonSelection.create({ label: 'A', runIds: [a] }), wire.ComparisonSelection.create({ label: 'B', runIds: [b] })] })).finish(),
        fetcher,
        signal,
      ),
    historyAnalysis: (moduleId: string, a: string, b: string, signal: AbortSignal) => request(routes.historyAnalysis,
      wire.AnalysisRequest.encode(wire.AnalysisRequest.create({ moduleId, sides: [
        wire.ComparisonSelection.create({ label: 'A', runIds: [a] }), wire.ComparisonSelection.create({ label: 'B', runIds: [b] }),
      ] })).finish(), fetcher, signal),
    /** Where to download one artifact; undefined when this client has no file for it. */
    artifactUrl: (runId: string, id: string, source?: doramcap.console.v1.IRunFileRequest): string | undefined => source ? runFilePath(runId, source) : artifactPath(runId, id),
    live: (consume: (status: Snapshot) => void, signal: AbortSignal, frame?: (frame: Frame) => void) => readLive(consume, signal, fetcher, frame),
  };
}
export const api = createClient();
export const cpuRecordingEnabled = (snapshot: Pick<Snapshot, 'recordings'> | null) =>
  snapshot?.recordings.some((output) => output.cpu) ?? false;
