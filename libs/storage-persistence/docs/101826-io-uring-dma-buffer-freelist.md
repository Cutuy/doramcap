# Idea: io_uring recording with a managed buffer freelist

Explore a high-throughput MCAP recording path using a fixed pool of reusable,
aligned host-memory buffers and asynchronous direct writes through io_uring.
The goal is bounded memory use, predictable CPU overhead, and fewer copies on
sustained high-bandwidth streams.

## Buffer lifecycle

```text
free → acquired → filled → submitted → completed → free
```

A managed freelist owns the available buffers. The recorder acquires buffers,
fills them with MCAP data, and submits writes. Submitted buffers remain owned
by the storage path until completion confirms they can be reused. Completed
buffers return to the freelist.

Pool size bounds memory and outstanding data. Exhaustion requires an explicit
policy for backpressure or dropped data, with visible loss counters. Collection
must retain its existing bounded handoff and timing independence.

## Storage direction

Evaluate io_uring with registered buffers and direct I/O so the storage device
can DMA from reusable host buffers while reducing page-cache copies and repeated
buffer-registration overhead. Actual copy behavior depends on the filesystem,
driver, buffer alignment, and hardware. MCAP framing and compression may still
require CPU work and copies before submission.

Keep MCAP compatibility, timestamps, and recovery properties. Define separately
when buffers become reusable and when recorded data is durable across power loss.

## Questions to revisit

- Can MCAP framing fill pooled buffers directly while preserving valid files?
- What buffer sizes and pool depth sustain target bandwidth within the memory budget?
- How should backpressure, slow storage, write failures, and shutdown behave?
- What durability cadence and crash-recovery window does recording require?
- How do throughput, tail latency, CPU cost, allocations, and copies compare with
  the current buffered writer on the target Jetson and NVMe device?

Reference: [MCAP asynchronous direct-I/O discussion](https://github.com/foxglove/mcap/discussions/1087).
