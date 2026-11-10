# CUDA slab details and results

Created 2026-11-09. [End-to-end flow](README.md).
Source: [support.rs](support.rs); hardware tests: [tests.rs](tests.rs).

The CPU handles MCAP framing, GPU readiness, submission and completion. Mapped
mode lets the GPU write host RAM directly; staged mode adds a device-to-host
copy on the same non-default stream. A slab becomes writable again only after
its last I/O reference is released. If GPU synchronization fails, the example
retains the allocation rather than freeing memory the GPU may still access.

Payload size must fit `u32`; payload, message count and slab count must be nonzero.
The writer owns its registration budget and segmentation; `prepare_slabs`
reports exhaustion. CUDA registration lasts for the pool's lifetime.

## Hardware checks

**Work items**

- [x] Register reusable slabs with CUDA and io_uring; preserve completion ordering.

**Validation**

- [x] All 14 cases pass on RTX 5070 Ti: exact readback and slab reuse, including mapped/staged 64-MiB payloads. The large-slab regression failed before the capacity fix.
- [x] Locked release compilation, focused Clippy, formatting and diff checks pass (code `188a0413`).

```sh
cargo test --release --locked -p storage-persistence --test cuda_mcap -- --ignored --nocapture --test-threads=1
```

Earlier ARM64 CPU/native worker checks passed (job 639); CUDA tests were ignored.
iGPU/Thor hardware remains unqualified. This documentation update changes no code.

## Benchmark

RTX 5070 Ti, i7-12700KF, Samsung 970 EVO 250 GB, ext4 about 91% full.
Code `798926cf`; median of three 1-GiB recordings per mode, 8-MiB messages,
four slabs. All 27 recordings across the benchmark's payload sizes passed readback.

| Mode | MiB/s | CPU seconds/GiB | CPU, one core |
|---|---:|---:|---:|
| Mapped slab | 108.228 | 0.495 | 5.351% |
| Device memory → slab | 98.763 | 0.510 | 4.727% |
| Pre-produced block I/O | 108.997 | 0.588 | 6.260% |

Timing includes production, CPU payload clearing, I/O drain and final sync;
setup and readback are excluded. CPU is process user+system time, not total
machine CPU. Storage throughput varied substantially, so these results establish
correct recording without a clear performance win. The capacity fix and this
documentation update did not rerun the benchmark.
