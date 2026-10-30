/** Validate packaged startup against a local Docker API stub; no Engine launch. */
import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { createServer as httpServer } from 'node:http';
import { createServer as netServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
const packageDir = resolve(process.argv[2]);
const root = await mkdtemp(join(tmpdir(), 'console-package-smoke-'));
const socket = join(root, 'docker.sock');
const api = httpServer((_request, response) => { response.writeHead(200, { 'content-type': 'application/json' }); response.end('{"ApiVersion":"1.45"}'); });
api.listen(socket); await once(api, 'listening');
const probe = netServer(); probe.listen(0, '127.0.0.1'); await once(probe, 'listening');
const address = probe.address(); assert.ok(address && typeof address !== 'string'); const port = address.port;
await new Promise<void>(ok => probe.close(() => ok()));
const origin = `http://127.0.0.1:${port}`;
await writeFile(join(root, 'secret'), 'test-secret', { mode: 0o600 });
await writeFile(join(root, 'console.pbtxt'), `listen_address: "127.0.0.1:${port}"\npublic_origin: "${origin}"\nproject_root: "${root}"\ndata_root: "${root}/data"\ndocker_socket: "${socket}"\naccess_secret_file: "${root}/secret"\nsystem_apps { cpu_monitor { image: "monitor:local" } }\n`);
const child = spawn(process.execPath, [`${packageDir}/server/dist/main.js`, join(root, 'console.pbtxt'), `${packageDir}/frontend`], { stdio: 'pipe' });
let diagnostics = ''; child.stderr.on('data', chunk => { diagnostics += chunk.toString(); });
try {
  let response: Response | undefined;
  for (let attempt = 0; attempt < 100; attempt++) {
    try { response = await fetch(`${origin}/api/current`, { headers: { authorization: `Basic ${Buffer.from('console:test-secret').toString('base64')}` } }); if (response.ok) break; } catch {}
    if (child.exitCode !== null) throw new Error(`packaged server exited: ${diagnostics}`);
    await delay(100);
  }
  assert.equal(response?.status, 200);
  assert.equal((await response!.json() as {phase: string}).phase, 'IDLE');
  const page = await fetch(origin, { headers: { authorization: `Basic ${Buffer.from('console:test-secret').toString('base64')}` } });
  assert.equal(page.status, 200); assert.match(await page.text(), /<div id="root">/);
  console.log('PASS: packaged startup, authenticated status and frontend asset');
} finally {
  child.kill('SIGTERM');
  if (child.exitCode === null && child.signalCode === null) await new Promise<void>((ok, fail) => {
    const timer = setTimeout(() => { child.kill('SIGKILL'); fail(new Error('server failed to stop')); }, 10_000);
    child.once('exit', () => { clearTimeout(timer); ok(); });
  });
  api.close(); await once(api, 'close');
  await rm(root, { recursive: true, force: true });
}
