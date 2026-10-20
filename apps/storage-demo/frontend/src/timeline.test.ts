import assert from "node:assert/strict";
import { test } from "node:test";
import { zoomTimeline } from "./timeline";
test("wheel zoom preserves the timestamp under the pointer", () => {
  for (const fraction of [0, 0.3, 1]) {
    const next = zoomTimeline(12, 40, fraction, -100);
    assert.ok(next.seconds < 12);
    assert.ok(
      Math.abs(
        40 -
          12 +
          fraction * 12 -
          (next.right - next.seconds + fraction * next.seconds),
      ) < 1e-10,
    );
    const restored = zoomTimeline(next.seconds, next.right, fraction, 100);
    assert.ok(Math.abs(restored.seconds - 12) < 1e-10);
    assert.ok(Math.abs(restored.right - 40) < 1e-10);
  }
});
test("zoom bounds scale and keeps the timeline above zero", () => {
  assert.equal(zoomTimeline(0.001, 40, 0.5, -1000).seconds, 0.001);
  assert.equal(zoomTimeline(60, 80, 0.5, 1000).seconds, 60);
  const next = zoomTimeline(12, 12, 1, 1000);
  assert.equal(next.right, next.seconds);
});
