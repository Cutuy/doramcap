import { establishSession } from '../frontend/src/session.js';
import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { createClient, ApiError, Phase } from '../frontend/src/api.js';
import { createConsoleServer } from '../server/src/http.js';
import type { ConsoleBackend } from '../server/src/native.js';
import { doramcap } from '../server/src/generated/console.js';
const wire = doramcap.console.v1;
const live = doramcap.console.live.v1;

// Exercise the browser's generated codecs against the real server route handling.
test(
  'browser and server agree on commands, selection presence, errors, live status and relayed frames',
  { timeout: 10_000 },
  async (t) => {
    // Arrange
    const selections: (string[] | undefined)[] = [];
    const id = '9007199254740993';
    const pastRun = 'a'.repeat(32);
    const snapshot = wire.Status.fromObject({
      phase: wire.Phase.RUNNING,
      runId: 'run',
      sequence: id,
      steps: [{ version: 1, session: 'run', executor: 'local', invocation: id, state: 1 }],
    });
    const bridgeFrame = live.Frame.encode({ stream: 'cpu_0', sequence: id, payload: Buffer.from([7]) }).finish();
    const backend: ConsoleBackend = {
      frames: (consume) => {
        consume(Buffer.from(bridgeFrame));
        return () => {};
      },
      prepare: async (input) => {
        const request = wire.PrepareRequest.decode(input);
        if (request.profiles.includes('journal-unavailable'))
          throw Object.assign(new Error('InvalidInput'), { check: 'configuration.rs:1', detail: 'raw journal failure' });
        selections.push(request.systemApps?.ids ?? undefined);
        return Buffer.from(wire.Review.encode({ preparedId: 'prepared', runId: 'run' }).finish());
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
      fileChanges: async function* () { throw new Error('Missing'); },
      openArtifact: async () => {
        throw new Error('Missing');
      },
      history: async (input) => {
        const query = wire.HistoryQuery.decode(input);
        return Buffer.from(wire.HistoryPage.encode({ entries: [{ runId: pastRun, startedAtMs: id as never, state: query.state }], nextCursor: `1:${pastRun}` }).finish());
      },
      historyRun: async (runId) => {
        if (runId !== pastRun) throw new Error('Missing');
        return Buffer.from(wire.HistoryDetail.encode({ entry: { runId }, status: snapshot }).finish());
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
    assert.equal((await client.prepare([])).preparedId, 'prepared');
    await client.prepare([], []);
    await client.prepare(['extra'], ['recorder']);
    assert.deepEqual(selections, [undefined, [], ['recorder']]);
    await assert.rejects(client.prepare(['journal-unavailable']), (error: unknown) => {
      assert.ok(error instanceof ApiError);
      assert.equal(error.code, 'InvalidInput');
      assert.equal(error.check, 'configuration.rs:1');
      assert.equal(error.detail, 'raw journal failure');
      return true;
    });
    assert.equal((await client.start('run', 'accepted')).runId, 'run');
    assert.equal((await client.stop('run')).accepted, true);
    await assert.rejects(
      client.start('run', 'reject'),
      (error: unknown) => error instanceof ApiError && error.code === 'PreflightRejected',
    );
    const current = await client.current();
    assert.equal(current.phase, Phase.RUNNING);
    assert.equal(String(current.sequence), id);

    // Act / Assert: history queries and past runs keep their typed fields and error codes.
    const past = await client.history({ state: wire.HistoryState.HISTORY_FAILED });
    assert.equal(String(past.entries[0].startedAtMs), id);
    assert.equal(past.entries[0].state, wire.HistoryState.HISTORY_FAILED);
    assert.equal(past.nextCursor, `1:${pastRun}`);
    assert.equal((await client.historyRun(pastRun)).status!.runId, 'run');
    await assert.rejects(
      client.historyRun('b'.repeat(32)),
      (error: unknown) => error instanceof ApiError && error.code === 'Missing',
    );
    await assert.rejects(
      client.history({ cursor: 'not-a-cursor' }),
      (error: unknown) => error instanceof ApiError && error.code === 'InvalidInput',
    );

    // Act / Assert: live status and relayed bridge frames keep exact IDs.
    const controller = new AbortController();
    let receive!: (status: Awaited<ReturnType<typeof client.current>>) => void;
    const received = new Promise<Awaited<ReturnType<typeof client.current>>>((resolve) => {
      receive = resolve;
    });
    let relay!: (frame: InstanceType<typeof live.Frame>) => void;
    const relayed = new Promise<InstanceType<typeof live.Frame>>((resolve) => {
      relay = resolve;
    });
    const streaming = client
      .live(receive, controller.signal, (frame) => {
        relay(frame as never);
        controller.abort();
      })
      .catch((error: unknown) => {
        if (!(error instanceof Error) || error.name !== 'AbortError') throw error;
      });
    t.after(() => controller.abort());
    const next = await received;
    assert.equal(next.phase, current.phase);
    assert.equal(String(next.steps[0].invocation), id);
    const frame = await relayed;
    assert.equal(frame.stream, 'cpu_0');
    assert.equal(String(frame.sequence), id);
    assert.deepEqual([...frame.payload], [7]);
    await streaming;
  },
);
