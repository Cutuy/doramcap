# Results and test

Created 2026-11-09. [Flow diagram](README.md) · [Source](src/support.rs) · [Tests](tests/cuda_mcap.rs).

| Mode | Payload path |
|---|---|
| `mapped` | GPU → pinned slab → file |
| `staged` | GPU device memory → async copy → pinned slab → file |
| `io-only` | Reuse a pre-produced slab → file |

Payload, messages and slabs must be nonzero; payload must fit `u32`.
`prepare_slabs` reports registration-budget exhaustion.

```sh
cargo test --release --locked -p gpudirect-demo --test cuda_mcap -- --ignored --nocapture --test-threads=1
```

**Verified:** 14 RTX 5070 Ti cases, including 64-MiB payloads, slab reuse and
independent full-byte MCAP readback; release build/Clippy passed (`188a0413`).
Earlier ARM64 CPU/native checks passed (job 639); iGPU/Thor remains unqualified.

**Benchmark:** RTX 5070 Ti, i7-12700KF, Samsung 970 EVO 250 GB, ext4 ~91% full;
three 1-GiB runs per mode, 8-MiB messages, four slabs (`798926cf`).

| Mode | Median MiB/s | CPU seconds/GiB | CPU, one core |
|---|---:|---:|---:|
| `mapped` | 108.228 | 0.495 | 5.351% |
| `staged` | 98.763 | 0.510 | 4.727% |
| `io-only` | 108.997 | 0.588 | 6.260% |

Timing includes production, CPU payload clearing, drain and sync; excludes setup
and readback. CPU measures process user+system time. Storage variability prevented
a clear speedup conclusion. These are earlier measurements, not rerun for docs.
