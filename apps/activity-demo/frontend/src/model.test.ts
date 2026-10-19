import { test } from "node:test";
import assert from "node:assert/strict";
import { Batch, TraceStore } from "./model.js";
function packet(sequence: number, records: object[], extra: object = {}) {
  return Batch.encode(
    Batch.fromObject({
      version: 1,
      session: "18446744073709551000",
      sequence,
      dictionary: ["Aster", "publish", "Birch", "receive", "text"],
      records,
      ...extra,
    }),
  ).finish();
}
test("keeps uint64 identities exact and preserves cross-thread causal parent", () => {
  const s = new TraceStore();
  s.ingest(
    packet(1, [
      {
        kind: 0,
        spanId: "9007199254740993",
        traceRoot: "9007199254740993",
        name: 1,
        threadName: 0,
        monotonicNs: 1000,
      },
      {
        kind: 4,
        spanId: "9007199254740993",
        traceRoot: "9007199254740993",
        name: 1,
        threadName: 0,
        monotonicNs: 2000,
      },
    ]),
  );
  s.ingest(
    packet(2, [
      {
        kind: 0,
        spanId: "9007199254740994",
        traceRoot: "9007199254740993",
        parentId: "9007199254740993",
        name: 3,
        threadName: 2,
        monotonicNs: 3000,
      },
    ]),
  );
  const spans = [...s.spans.values()];
  assert.equal(spans[1].parent, spans[0].id);
  assert.equal(spans[1].root, spans[0].root);
  assert.equal(spans[0].end, 0.000002);
  assert.match(spans[0].id, /9007199254740993$/);
});
test("deduplicates replay, exposes gaps, resets on process restart", () => {
  const s = new TraceStore();
  s.ingest(packet(5, []));
  assert.equal(s.ingest(packet(5, [])), false);
  s.ingest(packet(8, []));
  assert.equal(s.gaps, 2);
  s.ingest(packet(1, [], { session: "2" }));
  assert.equal(s.gaps, 0);
  assert.equal(s.lastSequence, 1n);
});
test("marks missing starts and bounds retained history", () => {
  const s = new TraceStore();
  s.ingest(packet(1, [{ kind: 4, spanId: 9, name: 1, threadName: 0 }]));
  assert.equal([...s.spans.values()][0].partial, true);
  for (let n = 2; n < 2200; n++)
    s.ingest(packet(n, [{ kind: 0, spanId: n, name: 1, threadName: 0 }]));
  assert.equal(s.spans.size, 2000);
});
test("rejects out-of-range string payloads", () => {
  const s = new TraceStore();
  assert.throws(
    () =>
      s.ingest(
        packet(1, [
          {
            kind: 1,
            name: 1,
            threadName: 0,
            fields: [{ name: 4, kind: 4, offset: 0, length: 5 }],
          },
        ]),
      ),
    /Invalid string/,
  );
});
