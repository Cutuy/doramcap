# CUDA slab example implementation

Created 2026-11-09. [Architecture](110926-cuda-mcap-slabs.md).

## M1 — Hardware example and test

**Work items**

- [x] Reuse workspace `cudarc` as a development dependency; keep production dependencies unchanged.
- [x] Register slab allocations once and retain their owner through unregister.
- [x] Use a non-default stream and a blocking readiness event before sealing.
- [x] Compare mapped output, staged device output and pre-produced-block I/O.
- [x] Report wall time, payload/file throughput, process user/system CPU and GPU time.
- [x] Verify every message and payload byte with the independent `mcap` reader.

**Validation**

- [ ] Release compilation and focused Clippy/format/boundary checks.
- [ ] Hardware roundtrip: page boundaries, arbitrary payload sizes and slab reuse.
- [ ] Repeat NVMe measurements; record hardware, commands and timing scope.
- [ ] ARM64 worker checks; report GPU/Thor checks separately.
- [ ] Scanner review, final PR validation record and lease release.

## Commands

```sh
cargo build --release --locked -p storage-persistence --example cuda_mcap
cargo test --release --locked -p storage-persistence --test cuda_mcap -- --ignored --nocapture
target/release/examples/cuda_mcap /absolute/path/output.mcap mapped 1048576 2048 4
```

Modes: `mapped`, `staged`, `io-only`. Output creation is exclusive. Slabs and
messages must be nonzero; payload size is limited by the kernel's `u32` launch
index. Registration capacity is derived from slab pages and writer buffers.
The example requires CUDA driver/runtime/NVRTC libraries and a Linux filesystem
supporting direct I/O and io_uring. Normal CPU-only CI leaves the hardware test
ignored. Build unprivileged; use test-executable elevation only when required.

## Results

Pending hardware execution. No throughput or platform-qualification claim yet.
