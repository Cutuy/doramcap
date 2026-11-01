import { establishSession } from '../frontend/src/session.js';
import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { createClient, ApiError, Phase } from '../frontend/src/api.js';
import { createConsoleServer } from '../server/src/http.js';
import type { ConsoleBackend } from '../server/src/native.js';
import { doramcap } from '../server/src/generated/console.js';
const wire = doramcap.console.v1;

// Exercise the browser's generated codecs against the real server route handling.
test(
  'browser and server agree on commands, selection presence, errors and live status',
  { timeout: 10_000 },
  async (t) => {
    // Arrange
    const selections: (string[] | undefined)[] = [];
    const id = '9007199254740993';
    const snapshot = wire.Status.fromObject({
      phase: wire.Phase.RUNNING,
      runId: 'run',
      sequence: id,
      steps: [{ version: 1, session: 'run', executor: 'local', invocation: id, state: 1 }],
    });
    const backend: ConsoleBackend = {
      prepare: async (input) => {
        const request = wire.PrepareRequest.decode(input);
        selections.push(request.systemApps?.ids ?? undefined);
        return Buffer.from(wire.Review.encode({ preparedId: request.directory, runId: 'run' }).finish());
      },
      start: async (input) => {
        const request = wire.StartRequest.decode(input);
        if (request.requestKey === 'reject') throw new Error('PreflightRejected');
        return Buffer.from(wire.StartResponse.encode({ runId: request.preparedId }).finish());
      },
      stop: async (input) => {
        assert.equal(wire.StopRequest.decode(input).runId, 'run');
        return Buffer.from(wire.StopResponse.encode({ accepted: true }).finish());
      },
      status: async () => Buffer.from(wire.Status.encode(snapshot).finish()),
      update: async () => ({
        data: Buffer.from(wire.Status.encode(snapshot).finish()),
        cursor: Buffer.alloc(0),
      }),
      openArtifact: async () => {
        throw new Error('Missing');
      },
      close: async () => {},
    };
    const server = createConsoleServer({
      backend,
      secret: 'secret',
      origin: 'http://console.test',
      assets: '/tmp',
    });
    server.listen(0, '127.0.0.1');
    t.after(() => {
      server.closeAllConnections();
      server.close();
    });
    await once(server, 'listening');
    const address = server.address();
    assert.ok(address && typeof address !== 'string');
    const base = `http://127.0.0.1:${address.port}`;
    let cookie = '';
    let cleanUrl = '';
    await establishSession({ hash: '#access=secret', pathname: '/', search: '' },
      { replaceState: (_data, _unused, url) => { cleanUrl = String(url); } },
      async (path, init) => {
        const response = await fetch(new URL(String(path), base), {
          ...init, headers: { ...init?.headers, origin: 'http://console.test' },
        });
        cookie = response.headers.get('set-cookie')?.split(';')[0] ?? '';
        return response;
      });
    assert.equal(cleanUrl, '/');
    assert.ok(cookie);
    const client = createClient((path, init) =>
      fetch(new URL(String(path), base), {
        ...init,
        headers: {
          ...init?.headers,
          origin: 'http://console.test',
          cookie,
        },
      }),
    );

    // Act / Assert
    assert.equal((await client.prepare('/projects/example', [])).preparedId, '/projects/example');
    await client.prepare('/projects/example', [], []);
    await client.prepare('/projects/example', ['extra'], ['recorder']);
    assert.deepEqual(selections, [undefined, [], ['recorder']]);
    assert.equal((await client.start('run', 'accepted')).runId, 'run');
    assert.equal((await client.stop('run')).accepted, true);
    await assert.rejects(
      client.start('run', 'reject'),
      (error: unknown) => error instanceof ApiError && error.code === 'PreflightRejected',
    );
    const current = await client.current();
    assert.equal(current.phase, Phase.RUNNING);
    assert.equal(String(current.sequence), id);

    // Act / Assert: a live frame uses the same message model and exact IDs.
    const controller = new AbortController();
    let receive!: (status: Awaited<ReturnType<typeof client.current>>) => void;
    const received = new Promise<Awaited<ReturnType<typeof client.current>>>((resolve) => {
      receive = resolve;
    });
    const streaming = client
      .live((next) => {
        receive(next);
        controller.abort();
      }, controller.signal)
      .catch((error: unknown) => {
        if (!(error instanceof Error) || error.name !== 'AbortError') throw error;
      });
    t.after(() => controller.abort());
    const next = await received;
    assert.equal(next.phase, current.phase);
    assert.equal(String(next.steps[0].invocation), id);
    await streaming;
  },
);
