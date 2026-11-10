# CUDA slab example implementation

Created 2026-11-09. [Architecture](README.md).

## M1 — Hardware example and test

**Work items**

- [x] Reuse workspace `cudarc` as a development dependency; keep production dependencies unchanged.
- [x] Register slab allocations once and retain their owner through unregister.
- [x] Use a non-default stream and a blocking readiness event before sealing.
- [x] Compare mapped output, staged device output and pre-produced-block I/O.
- [x] Report wall time, payload/file throughput, process user/system CPU and GPU time.
- [x] Verify every message and payload byte with the independent `mcap` reader.

**Validation**

- [x] Release compilation and focused all-feature Clippy/format/boundary checks.
- [x] Hardware roundtrip: page boundaries, arbitrary payload sizes and slab reuse.
- [x] Repeat NVMe measurements; record hardware, commands and timing scope.
- [x] ARM64 worker checks; GPU/Thor checks reported separately.
- [x] Scanner review; no source findings or policy reconciliation changes.
- [x] Final PR validation record.

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

Measured code revision: `798926cf73688514d2f5c0e4dfa16cf64edf821a`.
Host: Intel Core i7-12700KF, Linux 7.0.0-38-generic, RTX 5070 Ti 16 GB,
driver 595.104.02. Files: ext4 on the Samsung 970 EVO 250 GB root NVMe,
about 91% full before testing. Tests ran unprivileged.

All 12 hardware roundtrips passed: mapped, staged and storage-only modes with
1, 4,065, 65,543 and 1,048,576-byte payloads, 19 messages and three slabs.
Every decoded payload byte, sequence, topic and message count matched; all
slabs returned before CUDA unregister. The non-default stream assertion passed.

Benchmark: four slabs; three repetitions per mode; rotate mode order each
repetition. Each 64-KiB run records 4,096 messages (256 MiB); each 1-MiB run
records 1,024 messages (1 GiB); each 8-MiB run records 128 messages (1 GiB).
All 27 recordings passed independent full-payload readback. Temporary benchmark
files were removed after validation.

8-MiB messages, median of three repetitions:

| Mode | Payload MiB/s | Range MiB/s | CPU seconds/GiB | CPU, one core |
|---|---:|---:|---:|---:|
| GPU writes mapped slab | 108.228 | 98.569–127.768 | 0.495 | 5.351% |
| GPU device memory → async copy → slab | 98.763 | 94.864–101.890 | 0.510 | 4.727% |
| Pre-produced block → storage | 108.997 | 108.079–112.905 | 0.588 | 6.260% |

The throughput interval includes production, the existing CPU payload clearing,
slab submission, drain and final sync. CPU is process user+system time, normalized
to one core; columns are independent medians. CUDA-event intervals include the
kernel and, for staged output, the copy. Setup and readback are excluded.

The initial small recordings reached about 1.3 GiB/s. Later 1-GiB recordings
slowed to roughly 100 MiB/s in all modes, including storage-only output. This
variation prevents attributing the throughput differences to GPU mapping. The
experiment establishes correct GPU-produced slab recording without an explicit
copy in mapped mode; it does not establish a clear performance advantage on this
storage setup. No device firmware/cache-state cause was independently diagnosed.

Validation:

- `cargo test --release --locked -p storage-persistence --test cuda_mcap -- --ignored --nocapture`: passed.
- `cargo test --release --locked -p storage-persistence --all-features`: passed; hardware tests remain opt-in, and the existing 100-MiB test remained ignored.
- `cargo clippy --release --locked -p storage-persistence --all-targets --all-features --no-deps -- -D warnings`: passed.
- Formatting, diff checks and library boundaries: passed.
- GitHub Rust, Node and MCAP jobs: passed on x86_64 and ARM64 for the measured code revision.
- ARM64 WSL2 Pueue **639: Success**, complete log retrieved and checked; default/all-feature Rust and native worker checks passed. Snapshot: `/home/doramcap-ci/jobs/ff2ed9fbea8d467e8840258cf7d572ec`.
- ARM64 CUDA tests were ignored. No iGPU/Thor hardware qualification, GPU-fault injection or low-rate pacing benchmark was performed.

Reproduce one 8-MiB run with the release executable:

```sh
target/release/examples/cuda_mcap /absolute/path/mapped.mcap mapped 8388608 128 4
target/release/examples/cuda_mcap /absolute/path/staged.mcap staged 8388608 128 4
target/release/examples/cuda_mcap /absolute/path/io.mcap io-only 8388608 128 4
```

## M2 — Example directory consolidation · 2026-11-10

**Work items**

- [x] Group source, hardware tests, design, SVG and evidence in this directory.
- [x] Preserve the `cuda_mcap` example/test commands with explicit Cargo paths.
- [x] Remove the raw CSV and its links; retain the benchmark summary above.

**Validation**

- [x] Cargo metadata resolves both relocated targets; release compilation check passes.
- [x] Shared CUDA source remains byte-for-byte unchanged; local documentation links resolve.
- [x] Formatting and diff checks pass.
- [x] Execute the relocated CUDA hardware test: all 12 cases passed on the RTX 5070 Ti.

The relocation preserves the measured code and Cargo target names. This follow-up
reran the local hardware test and focused compilation checks. The benchmark and
ARM64 worker results above remain the earlier behavior qualification.
