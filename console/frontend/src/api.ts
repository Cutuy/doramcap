import { MEDIA_TYPE, LIMITS, isErrorCode, type ErrorCode } from './generated/transport';
import { readLive } from './live';

export * from './protocol';
import { wire, routes, type Snapshot } from './protocol';
export class ApiError extends Error {
  constructor(public code: ErrorCode) {
    super(code);
  }
}

async function request<T>(
  endpoint: { path: string; method: string; status: number; response: { decode(bytes: Uint8Array): T } },
  body: Uint8Array | undefined,
  fetcher: typeof fetch,
): Promise<T> {
  const response = await fetcher(endpoint.path, {
    method: endpoint.method,
    body: body ? new Blob([new Uint8Array(body)]) : undefined,
    headers: { accept: MEDIA_TYPE, ...(body ? { 'content-type': MEDIA_TYPE } : {}) },
    credentials: 'same-origin',
  });
  if (response.headers.get('content-type')?.split(';')[0].trim() !== MEDIA_TYPE)
    throw new Error('Invalid API response type');
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (bytes.length > LIMITS.frameBytes) throw new Error('API response exceeds limit');
  if (!response.ok) {
    const error = routes.error.decode(bytes);
    throw new ApiError(isErrorCode(error.code) ? error.code : 'Internal');
  }
  if (response.status !== endpoint.status) throw new Error('Invalid API response status');
  return endpoint.response.decode(bytes);
}
/** Create the browser client with an explicit transport for HTTP integration tests. */
export function createClient(fetcher: typeof fetch = fetch) {
  return {
    prepare: (directory: string, profiles: string[], systemApps?: string[]) =>
      request(
        routes.prepare,
        routes.prepare.request
          .encode(
            wire.PrepareRequest.create({
              directory,
              profiles,
              ...(systemApps === undefined
                ? {}
                : { systemApps: wire.SystemAppSelection.create({ ids: systemApps }) }),
            }),
          )
          .finish(),
        fetcher,
      ),
    start: (preparedId: string, requestKey: string) =>
      request(
        routes.start,
        routes.start.request.encode(wire.StartRequest.create({ preparedId, requestKey })).finish(),
        fetcher,
      ),
    stop: (runId: string) =>
      request(routes.stop, routes.stop.request.encode(wire.StopRequest.create({ runId })).finish(), fetcher),
    current: () => request(routes.current, undefined, fetcher),
    live: (consume: (status: Snapshot) => void, signal: AbortSignal) => readLive(consume, signal, fetcher),
  };
}
export const api = createClient();
export const cpuRecordingEnabled = (snapshot: Pick<Snapshot, 'recordings'> | null) =>
  snapshot?.recordings.some((output) => output.cpu) ?? false;
