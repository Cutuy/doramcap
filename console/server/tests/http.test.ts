import { sessionCookie } from './session.js';
import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { request as httpRequest } from 'node:http';
import { createConsoleServer } from '../src/http.js';
import { live, wire, type ConsoleBackend } from '../src/native.js';
import { routes, MEDIA_TYPE, LIMITS } from '../src/generated/transport.js';

test('authentication, Origin, size and field validation precede native mutations', async () => {
  // Arrange
  let starts = 0; let stops = 0;
  const selections: (string[] | undefined)[] = [];
  const native: ConsoleBackend = {
    update: async () => ({ data: Buffer.alloc(0), cursor: Buffer.alloc(0) }),
    frames: () => () => {},
    close: async () => {}, openArtifact: async () => { throw new Error('Missing'); },
    prepare: async input => { selections.push(wire.PrepareRequest.decode(input).systemApps?.ids ?? undefined); return Buffer.from(wire.Review.encode({ runId: 'run', preparedId: 'run' }).finish()); },
    start: async input => { const key = wire.StartRequest.decode(input).requestKey; if (key === 'stopped') throw new Error('Stopped'); if (key === 'rejected') throw new Error('PreflightRejected'); if (key === 'closing') throw new Error('Closing'); starts++; return Buffer.from(wire.StartResponse.encode({ runId: 'run' }).finish()); },
    stop: async () => { stops++; return Buffer.from(wire.StopResponse.encode({ accepted: true }).finish()); },
    status: async () => Buffer.from(wire.Status.encode({ phase: wire.Phase.RUNNING, sequence: '9007199254740993' as never }).finish()),
  };
  const assets = await mkdtemp(join(tmpdir(), 'console-http-'));
  await writeFile(join(assets, 'index.html'), 'console');
  const server = createConsoleServer({ backend: native, secret: 'secret', origin: 'http://console.test', assets });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  const address = server.address(); assert.ok(address && typeof address !== 'string');
  const base = `http://127.0.0.1:${address.port}`;
  const auth = await sessionCookie(base);
  const post = (path: string, data: Uint8Array, origin = 'http://console.test') => fetch(base + path, { method: 'POST', headers: { cookie: auth, origin, 'content-type': MEDIA_TYPE }, body: new Blob([new Uint8Array(data)]) });
  const start = (key: string) => wire.StartRequest.encode({preparedId:'run',requestKey:key}).finish();
  try {
    // Act / Assert
    assert.equal((await fetch(base + '/')).status, 200);
    assert.equal((await fetch(base + '/api/current')).status, 401);
    for (const headers of [{ origin: 'http://evil.test', authorization: 'Bearer secret' },
      { origin: 'http://console.test', authorization: 'Bearer wrong' }])
      assert.equal((await fetch(base + '/api/session', { method: 'POST', headers })).status, 403);
    assert.equal((await fetch(base + '/api/current', { headers: { authorization: 'Basic bad' } })).status, 401);
    assert.equal((await post(routes.start.path, start('key'), 'http://evil.test')).status, 403);
    assert.equal((await post(routes.start.path, Uint8Array.of(0xff))).status, 400);
    assert.equal((await post(routes.stop.path, wire.StopRequest.encode({}).finish())).status, 400);
    assert.equal((await post(routes.start.path, new Uint8Array(LIMITS.bodyBytes + 1))).status, 400);
    assert.equal((await fetch(base + routes.start.path, { method:'POST', headers:{cookie:auth,origin:'http://console.test','content-type':'application/json'},body:'{}' })).status,400);
    const chunkedStatus = await new Promise<number>((resolve, reject) => {
      const request = httpRequest(base + '/api/start', { method: 'POST', headers: { cookie: auth, origin: 'http://console.test', 'content-type': MEDIA_TYPE, 'transfer-encoding': 'chunked' } }, response => { response.resume(); response.on('end', () => resolve(response.statusCode!)); });
      request.on('error', reject);
      request.write('x'.repeat(65536)); request.end('x');
    });
    assert.equal(chunkedStatus, 400);
    assert.equal(starts, 0); assert.equal(stops, 0);
    assert.equal((await post(routes.prepare.path, wire.PrepareRequest.encode({directory:'/tmp',profiles:[]}).finish())).status, 200);
    assert.equal((await post(routes.prepare.path, wire.PrepareRequest.encode({directory:'/tmp',profiles:[],systemApps:{ids:[]}}).finish())).status, 200);
    assert.equal((await post(routes.prepare.path, wire.PrepareRequest.encode({directory:'/tmp',profiles:[],systemApps:{ids:['cpu-monitor']}}).finish())).status, 200);
    assert.equal((await post(routes.prepare.path, wire.PrepareRequest.encode({directory:'/tmp',profiles:Array.from({length:LIMITS.selectionLength+1},()=> 'profile')}).finish())).status,400);
    assert.equal((await post(routes.prepare.path, wire.PrepareRequest.encode({directory:'/tmp',profiles:[],systemApps:{ids:['']}}).finish())).status,400);
    assert.deepEqual(selections, [undefined, [], ['cpu-monitor']]);
    assert.equal((await post(routes.start.path, start('key'))).status, 202);
    assert.equal((await post(routes.stop.path, wire.StopRequest.encode({runId:'run'}).finish())).status, 200);
    assert.equal(starts, 1); assert.equal(stops, 1);
    const stopped = await post(routes.start.path, start('stopped'));
    assert.equal(stopped.status, 409);
    assert.equal(wire.ConsoleError.decode(new Uint8Array(await stopped.arrayBuffer())).code, 'Stopped');
    const rejected = await post(routes.start.path, start('rejected'));
    assert.equal(rejected.status, 409);
    assert.equal(wire.ConsoleError.decode(new Uint8Array(await rejected.arrayBuffer())).code, 'PreflightRejected');
    const closing = await post(routes.start.path, start('closing'));
    assert.equal(closing.status, 503);
    assert.equal(wire.ConsoleError.decode(new Uint8Array(await closing.arrayBuffer())).code, 'Closing');
    const current = await fetch(base + '/api/current', { headers: { cookie: auth } });
    assert.equal(String(wire.Status.decode(new Uint8Array(await current.arrayBuffer())).sequence), '9007199254740993');
    assert.equal((await fetch(base + '/api/nope', { headers: { cookie: auth } })).status, 404);
    assert.equal((await fetch(base + '/', { headers: { cookie: auth } })).status, 200);
  } finally {
    server.closeAllConnections(); server.close(); await once(server, 'close');
    await rm(assets, { recursive: true, force: true });
  }
});

test('live feed forwards binary Rust frames with opaque cursors at a bounded cadence', async () => {
  // Arrange
  let calls = 0; let statuses = 0; let subscribers = 0;
  const cursors: number[] = [];
  const bridgeFrame = Buffer.from(live.Frame.encode({ stream: 'cpu-monitor_0', sequence: 7, payload: Buffer.from([1, 2, 3]) }).finish());
  const native: ConsoleBackend = {
    frames: consume => { subscribers++; consume(bridgeFrame); return () => { subscribers--; }; },
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
  const auth = await sessionCookie(new URL(url).origin);
  const controller = new AbortController();
  try {
    // Act / Assert: authentication and Origin checks still precede streaming.
    assert.equal((await fetch(url)).status,401);
    assert.equal((await fetch(url,{headers:{cookie:auth,origin:'http://other.test'}})).status,403);
    const response = await fetch(url,{headers:{cookie:auth},signal:controller.signal});
    assert.equal(response.headers.get('content-type'),'application/x-protobuf');
    const reader = response.body!.getReader();
    let pending = Buffer.alloc(0); let frames = 0; const relayed: Buffer[] = [];
    const started = performance.now();
    while (frames < 3) {
      const {value,done} = await reader.read(); assert.equal(done,false);
      pending = Buffer.concat([pending,Buffer.from(value!)]);
      while(pending.length>=4 && pending.length>=4+pending.readUInt32LE(0)) {
        const length=pending.readUInt32LE(0);
        const message = wire.LiveMessage.decode(pending.subarray(4,4+length));
        if (message.frame?.length) relayed.push(Buffer.from(message.frame));
        else { assert.equal(wire.Status.decode(message.status!).sequence.toString(),'9007199254740993'); frames++; }
        pending=pending.subarray(4+length);
      }
    }
    // Assert: bridge frames are relayed byte-for-byte beside status updates.
    assert.deepEqual(relayed, [bridgeFrame]);
    // Assert: no JSON/status round trip; the native cursor is passed back unchanged.
    assert.equal(statuses,0); assert.deepEqual(cursors.slice(0,3),[0,1,2]);
    assert.ok(performance.now()-started>=150,'feed must not spin unbounded');
    const others = await Promise.all(Array.from({length:LIMITS.liveStreams-1}, () => fetch(url,{headers:{cookie:auth},signal:controller.signal})));
    assert.ok(others.every(stream => stream.status === 200));
    const busy = await fetch(url,{headers:{cookie:auth}});
    assert.equal(busy.status,503);
    assert.equal(wire.ConsoleError.decode(new Uint8Array(await busy.arrayBuffer())).code,'Busy');
    controller.abort(); await reader.cancel().catch(()=>{});
    // Assert: closed browser streams unsubscribe from bridge frames.
    for (let i = 0; i < 100 && subscribers; i++) await new Promise(resolve => setTimeout(resolve, 10));
    assert.equal(subscribers, 0);
  } finally { controller.abort(); server.closeAllConnections(); server.close(); await once(server,'close'); }
});
