import { api, routes, wire, type Review, type Snapshot } from './api';

// Checked by the frontend build; this function is never executed.
export function checkBrowserContract() {
  const review: Promise<Review> = api.prepare('/projects/example', [], []);
  const current: Promise<Snapshot> = api.current();
  void [review, current];
  // @ts-expect-error Profile selections are strings.
  api.prepare('/projects/example', [42]);
  // @ts-expect-error A review response cannot be used as a status response.
  const wrongResponse: Promise<Snapshot> = api.prepare('/projects/example', []);
  // @ts-expect-error Start has preparedId/requestKey fields, not a StopRequest.
  routes.start.request.encode(wire.StopRequest.create({ runId: 'run' }));
  // @ts-expect-error The generated phase is an enum, not an unchecked string.
  const phase: Snapshot['phase'] = 'RUNNING';
  void [wrongResponse, phase];
}
