import assert from "node:assert/strict";
import { TraceStore } from "../frontend/src/model.ts";
const base = process.env.DEMO_URL ?? "http://127.0.0.1:8088";
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const status = async () => {
  const r = await fetch(`${base}/api/status`);
  assert.equal(r.status, 200);
  return r.json();
};
const control = async (paused: boolean) => {
  const r = await fetch(`${base}/api/control`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ paused }),
  });
  assert.equal(r.status, 200);
};
assert.match(await (await fetch(base)).text(), /Activity/);
const store = new TraceStore();
let failure: unknown;
function connect() {
  const ws = new WebSocket(base.replace("http", "ws") + "/stream");
  ws.binaryType = "arraybuffer";
  ws.onmessage = (e) => {
    try {
      if (e.data instanceof ArrayBuffer) store.ingest(new Uint8Array(e.data));
    } catch (error) {
      failure = error;
    }
  };
  return ws;
}
let ws = connect();
try {
  await control(false);
  await sleep(5500);
  if (failure) throw failure;
  const receives = [...store.spans.values()].filter(
    (s) => s.name === "receive",
  );
  assert.ok(receives.length >= 6);
  const threads = new Set(receives.map((s) => s.tid));
  assert.equal(threads.size, 4);
  for (const receive of receives) {
    const parent = store.spans.get(receive.parent);
    if (!parent) continue;
    assert.equal(parent.name, "publish");
    assert.equal(parent.root, receive.root);
    assert.notEqual(parent.worker, receive.worker);
    assert.equal(parent.fields["message.id"], receive.fields["message.id"]);
  }
  assert.ok(
    receives.some((r) => store.spans.get(r.parent)?.name === "publish"),
  );
  assert.ok(store.events.some((e) => e.fields.phase === "publish"));
  assert.ok(store.events.some((e) => e.fields.phase === "receive"));
  await control(true);
  assert.equal((await status()).paused, true);
  await sleep(2000);
  const before = await status();
  await sleep(1800);
  assert.equal((await status()).published, before.published);
  assert.equal(
    (await fetch(`${base}/api/greet`, { method: "POST" })).status,
    200,
  );
  await sleep(1800);
  assert.ok((await status()).published > before.published);
  ws.close();
  const seq = store.lastSequence;
  ws = connect();
  await sleep(500);
  assert.ok(store.lastSequence >= seq);
  assert.equal(store.gaps, 0);
  const final = await status();
  assert.equal(final.mailbox_drops, 0);
  assert.equal(final.capture_drops, 0);
  assert.equal(final.field_losses, 0);
  console.log(
    `Live check passed: ${store.received} records, ${threads.size} worker threads, causal pub/sub, controls, replay; MCAP ${final.recording}`,
  );
} finally {
  ws.close();
  await control(false);
}
