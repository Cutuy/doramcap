import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { request as httpRequest } from 'node:http';
import { createConsoleServer } from '../src/http.js';
import { wire, type ConsoleBackend } from '../src/native.js';

test('authentication, Origin, size and field validation precede native mutations', async () => {
  // Arrange
  let starts = 0; let stops = 0;
  const selections: (string[] | undefined)[] = [];
  const native: ConsoleBackend = {
    update: async () => ({ data: Buffer.alloc(0), cursor: Buffer.alloc(0) }),
    close: async () => {}, openArtifact: async () => { throw new Error('Missing'); },
    prepare: async input => { selections.push(wire.PrepareRequest.decode(input).systemApps?.ids ?? undefined); return Buffer.from(wire.Review.encode({ runId: 'run', preparedId: 'run' }).finish()); },
    start: async input => { if (wire.StartRequest.decode(input).requestKey === 'stopped') throw new Error('Stopped'); starts++; return Buffer.from(wire.StartResponse.encode({ runId: 'run' }).finish()); },
    stop: async () => { stops++; return Buffer.from(wire.StopResponse.encode({ accepted: true }).finish()); },
    status: async () => Buffer.from(wire.Status.encode({ phase: wire.Phase.RUNNING, sequence: '9007199254740993' as never }).finish()),
  };
  const assets = await mkdtemp(join(tmpdir(), 'console-http-'));
  await writeFile(join(assets, 'index.html'), 'console');
  const server = createConsoleServer({ backend: native, secret: 'secret', origin: 'http://console.test', assets });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  const address = server.address(); assert.ok(address && typeof address !== 'string');
  const base = `http://127.0.0.1:${address.port}`;
  const auth = `Basic ${Buffer.from('console:secret').toString('base64')}`;
  const post = (path: string, data: string, origin = 'http://console.test') => fetch(base + path, { method: 'POST', headers: { authorization: auth, origin, 'content-type': 'application/json' }, body: data });
  try {
    // Act / Assert
    assert.equal((await fetch(base + '/')).status, 401);
    assert.equal((await fetch(base + '/api/current', { headers: { authorization: 'Basic bad' } })).status, 401);
    assert.equal((await post('/api/start', '{"preparedId":"run","requestKey":"key"}', 'http://evil.test')).status, 403);
    assert.equal((await post('/api/start', '{"preparedId":"run","requestKey":"key","extra":1}')).status, 400);
    assert.equal((await post('/api/stop', '{"runId":"run","extra":1}')).status, 400);
    assert.equal((await post('/api/start', JSON.stringify({ preparedId: 'x'.repeat(65536), requestKey: 'key' }))).status, 400);
    const chunkedStatus = await new Promise<number>((resolve, reject) => {
      const request = httpRequest(base + '/api/start', { method: 'POST', headers: { authorization: auth, origin: 'http://console.test', 'content-type': 'application/json', 'transfer-encoding': 'chunked' } }, response => { response.resume(); response.on('end', () => resolve(response.statusCode!)); });
      request.on('error', reject);
      request.write('x'.repeat(65536)); request.end('x');
    });
    assert.equal(chunkedStatus, 400);
    assert.equal(starts, 0); assert.equal(stops, 0);
    assert.equal((await post('/api/prepare', '{"directory":"/tmp","profiles":[]}')).status, 200);
    assert.equal((await post('/api/prepare', '{"directory":"/tmp","profiles":[],"systemApps":[]}')).status, 200);
    assert.equal((await post('/api/prepare', '{"directory":"/tmp","profiles":[],"systemApps":["cpu-monitor"]}')).status, 200);
    assert.equal((await post('/api/prepare', '{"directory":"/tmp","profiles":[],"systemApps":null}')).status, 400);
    assert.equal((await post('/api/prepare', '{"directory":"/tmp","profiles":[],"systemApps":[42]}')).status, 400);
    assert.deepEqual(selections, [undefined, [], ['cpu-monitor']]);
    assert.equal((await post('/api/start', '{"preparedId":"run","requestKey":"key"}')).status, 202);
    assert.equal((await post('/api/stop', '{"runId":"run"}')).status, 200);
    assert.equal(starts, 1); assert.equal(stops, 1);
    const stopped = await post('/api/start', '{"preparedId":"run","requestKey":"stopped"}');
    assert.equal(stopped.status, 409);
    assert.equal((await stopped.json() as {error:{code:string}}).error.code, 'Stopped');
    const current = await fetch(base + '/api/current', { headers: { authorization: auth } });
    assert.equal((await current.json() as { sequence: string }).sequence, '9007199254740993');
    assert.equal((await fetch(base + '/api/nope', { headers: { authorization: auth } })).status, 404);
    assert.equal((await fetch(base + '/', { headers: { authorization: auth } })).status, 200);
  } finally {
    server.closeAllConnections(); server.close(); await once(server, 'close');
    await rm(assets, { recursive: true, force: true });
  }
});

test('live feed forwards binary Rust frames with opaque cursors at a bounded cadence', async () => {
  // Arrange
  let calls = 0; let statuses = 0;
  const cursors: number[] = [];
  const native: ConsoleBackend = {
    close: async () => {}, openArtifact: async () => { throw new Error('Missing'); },
    prepare: async () => Buffer.alloc(0), start: async () => Buffer.alloc(0), stop: async () => Buffer.alloc(0),
    status: async () => { statuses++; return Buffer.alloc(0); },
    update: async cursor => {
      cursors.push(cursor[0] ?? 0); calls++;
      return { data: Buffer.from(wire.Status.encode(wire.Status.fromObject({phase:wire.Phase.RUNNING,runId:'run',sequence:'9007199254740993',steps:[{step:'x'.repeat(200000)}]})).finish()), cursor: Buffer.from([calls]) };
    },
  };
  const server = createConsoleServer({backend:native,secret:'secret',origin:'http://console.test',assets:'/tmp'});
  server.listen(0,'127.0.0.1'); await once(server,'listening');
  const address = server.address(); assert.ok(address && typeof address !== 'string');
  const url = `http://127.0.0.1:${address.port}/api/live`;
  const auth = `Basic ${Buffer.from('console:secret').toString('base64')}`;
  const controller = new AbortController();
  try {
    // Act / Assert: authentication and Origin checks still precede streaming.
    assert.equal((await fetch(url)).status,401);
    assert.equal((await fetch(url,{headers:{authorization:auth,origin:'http://other.test'}})).status,403);
    const response = await fetch(url,{headers:{authorization:auth},signal:controller.signal});
    assert.equal(response.headers.get('content-type'),'application/x-protobuf');
    const reader = response.body!.getReader();
    let pending = Buffer.alloc(0); let frames = 0;
    const started = performance.now();
    while (frames < 3) {
      const {value,done} = await reader.read(); assert.equal(done,false);
      pending = Buffer.concat([pending,Buffer.from(value!)]);
      while(pending.length>=4 && pending.length>=4+pending.readUInt32LE(0)) {
        const length=pending.readUInt32LE(0);
        assert.equal(wire.Status.decode(pending.subarray(4,4+length)).sequence.toString(),'9007199254740993');
        pending=pending.subarray(4+length); frames++;
      }
    }
    // Assert: no JSON/status round trip; the native cursor is passed back unchanged.
    assert.equal(statuses,0); assert.deepEqual(cursors.slice(0,3),[0,1,2]);
    assert.ok(performance.now()-started>=150,'feed must not spin unbounded');
    controller.abort(); await reader.cancel().catch(()=>{});
  } finally { controller.abort(); server.closeAllConnections(); server.close(); await once(server,'close'); }
});
