/** Local deterministic UI fixture; it never connects to Docker. */
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, resolve, sep } from 'node:path';
const assets = resolve('console/frontend/dist');
const runId = 'a'.repeat(32);
const base = { runId, sequence: '7', lostEvents: '0', truncatedSteps: '0', overflowSteps: '0', progressIncomplete: false, cpuAvailable: false, reportAvailable: false, monitorReady: false, outcome: null, steps: [] as object[] };
const steps = [
  { step: 'PrepareRun', elapsedNs: '900000000', durationNs: '900000000' },
  { step: 'PrepareSystemApps', elapsedNs: '700000000', durationNs: '400000000', parent: '1' },
  { step: 'PrepareOverlays', elapsedNs: '1300000000', durationNs: '400000000' },
  { step: 'LaunchProjects', elapsedNs: '2800000000', durationNs: '1500000000' },
  ...Array.from({ length: 12 }, (_, index) => ({ step: 'ObserveProjects', elapsedNs: String(3500000000 + index * 500000000), durationNs: '15000000' })),
  { step: 'InspectCompose', elapsedNs: '9200000000', durationNs: '8000000', parent: '15' },
].map((step, index) => ({ session: 'fixture', executor: 'local', invocation: String(index + 1), state: 'SUCCEEDED', ...step, step: `workload_docker::${step.step}` }));
const server = createServer(async (req, res) => {
  const url = new URL(req.url ?? '/', 'http://localhost');
  const fixture = url.searchParams.get('fixture');
  if (fixture) res.setHeader('set-cookie', `fixture=${fixture}; Path=/; SameSite=Strict`);
  const current = fixture ?? /(?:^|; )fixture=([^;]+)/.exec(req.headers.cookie ?? '')?.[1] ?? 'project';
  if (url.pathname === '/api/current') {
    const status = ['run', 'stopping', 'starting', 'loss'].includes(current) ? { ...base, phase: current === 'stopping' ? 'STOPPING' : current === 'starting' ? 'STARTING' : 'RUNNING', monitorReady: current !== 'starting', progressIncomplete: current === 'loss', lostEvents: current === 'loss' ? '1' : '0', steps: current === 'starting' ? [{ ...steps[0], state: 'STARTED', elapsedNs: '100000000', durationNs: '0' }] : steps } : current === 'result' ? { ...base, phase: 'FINISHED', cpuAvailable: true, reportAvailable: true, outcome: { execution: 'Completed', writersStopped: true, recordingFinalized: true, boundaryClean: true, eventStreamHealthy: true } } : current === 'blocked' ? { ...base, phase: 'BLOCKED', cpuAvailable: true, reportAvailable: true, outcome: { execution: 'Failed', writersStopped: true, recordingFinalized: true, boundaryClean: false, eventStreamHealthy: true } } : current === 'interrupted' ? { ...base, phase: 'BLOCKED' } : current === 'unavailable' ? { ...base, phase: 'FINISHED', outcome: { execution: 'Failed', writersStopped: true, recordingFinalized: false, boundaryClean: true, eventStreamHealthy: true } } : { ...base, phase: 'IDLE', runId: '' };
    res.writeHead(200, { 'content-type': 'application/json' }); res.end(JSON.stringify(status)); return;
  }
  if (url.pathname === '/api/prepare' && req.method === 'POST') {
    res.writeHead(200, { 'content-type': 'application/json' }); res.end(JSON.stringify({ preparedId: runId, runId, availableProfiles: ['extra-worker'], selectedProfiles: [], services: [{ name: 'worker', imageId: 'sha256:0123456789abcdef' }], warnings: [] })); return;
  }
  const target = resolve(assets, `.${url.pathname === '/' ? '/index.html' : url.pathname}`);
  if (!target.startsWith(assets + sep)) { res.writeHead(404); res.end(); return; }
  try { const bytes = await readFile(target); const mime: Record<string,string> = { '.html':'text/html', '.js':'text/javascript', '.css':'text/css' }; res.writeHead(200, { 'content-type': mime[extname(target)] ?? 'application/octet-stream' }); res.end(bytes); }
  catch { res.writeHead(404); res.end(); }
});
server.listen(4179, '127.0.0.1', () => console.log('UI fixture listening on 127.0.0.1:4179'));
