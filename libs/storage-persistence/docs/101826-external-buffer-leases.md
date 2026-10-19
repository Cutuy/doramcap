# External-buffer leases: gated feature plan

Status: planned separately from the managed-buffer writer. The expanded wire
layout SVG remains a review draft outside the repository. Implementation is paused.

## Design

The application adapter supplies immutable byte ranges and an owning guard.
The storage layer holds guards until every referencing request retires, including
retries and cancellation. Dora-specific ownership stays in its adapter.

All paths persist complete MCAP Messages containing valid serialized Protobuf.
For an external blob, the format adapter emits the field tag and length around
the borrowed bytes. Persist blob contents and sufficient schema information for
ordinary decoding. Existing serialized messages can be forwarded as byte spans.
Small and large managed messages retain the base writer's reserve/encode path.

The format adapter assembles ordered byte spans; the storage planner assigns
file ranges. Eligible spans use vectored writes or separate writes at explicit
file offsets. Submission batches may complete out of order. Fixed-buffer
registration is a distinct resource contract from keeping a SHM owner alive.

Direct I/O requires compatible memory addresses, lengths and file offsets.
Managed head/tail buffers can copy edge bytes when a compatible aligned middle
exists. Valid MCAP padding between records can adjust placement. Arbitrary
padding inside a Protobuf payload is excluded. Incompatible layouts use an
explicit copy policy or return a clear error under require-borrow policy.

## Gate rules

Follow the evidence and regression rules in the [base plan](101826-mcap-writer-design.md).
Require the base M7 gate before starting this feature. Complete E1–E5 in order;
check each milestone only after its acceptance checks pass on x86 and ARM64.
Record commands, commit, worker job result, counters and failures per milestone.
A required direct-I/O environment must be available for E4 qualification.

### E1 — external ownership with copy fallback

- [ ] Accept an external lease, copy into a managed reservation, release the
  external owner after copying, and reuse the established persistence path.
- Gate: exact MCAP/Protobuf payload equality for SHM, large managed and combined
  small messages; guard-drop counters show release after copy; dropped/failed
  reservations release once; no leaked owners under exhaustion or shutdown.
  Record copied bytes and allocations. Verify serialized input needs no decode.

### E2 — borrowed spans with buffered file I/O

- [ ] Add bounded span/request metadata and retain external guards through I/O.
  Use ordinary buffered file I/O to validate ownership and byte ordering first.
- Gate: byte-identical logical messages versus E1; multi-span fields and suffixes;
  out-of-order completions, partial writes, retries and cancellation. Poison or
  recycle source blocks only after retirement to detect premature reuse. Verify
  no application payload copy on this path; account for kernel buffering separately.

### E3 — Dora SHM adapter and pressure limits

- [ ] Retain Dora's actual received backing owner; bound retained blocks/bytes.
  Expose force-copy, prefer-borrow and require-borrow policies with explicit
  exhaustion behavior and counters.
- Gate: real producer/consumer reuse stress with delayed I/O; payload hashes;
  sender exit, receiver shutdown and source-pool pressure. Verify retained-byte
  limits and that every owner is eventually released. Measure allocation and
  copy counts using the actual Dora transport on both hosts.

### E4 — aligned borrowed direct I/O

- [ ] Select eligible aligned spans; assemble managed heads/tails when compatible;
  apply full-copy fallback for incompatible layouts. Keep per-request buffer
  registration/pinning costs visible; bound any external registration cache.
- Gate: real direct I/O with aligned and deliberately misaligned source slices,
  file offsets and lengths; tiny prefixes, tails and multiple blobs. Confirm
  independent decoding and payload hashes for every path; require-borrow rejects
  ineligible layouts. Exercise partial errors and registration teardown while
  requests are outstanding. Record borrowed, edge-copy and fallback byte counts.

### E5 — mixed-workload qualification

- [ ] Combine SHM, large managed and small combined records with rotation and
  production flush/close policies.
- Gate: long-running bounded-memory test, source-pool reuse, failures mid-record,
  slow storage and clean shutdown. Decode every finalized file. Compare against
  force-copy using identical input: throughput, CPU, p99 latency, retained bytes,
  copy bytes, allocation counts and losses. Set budgets before measurement;
  choose default policy from results. Target hardware validates DMA performance.

## Follow-up

- [ ] Discover actual direct-I/O alignment and validate configuration at startup;
  initial supported configurations are 4 KiB default and 512 B optional.
- [ ] Consider broader external-memory providers after Dora ownership is proven.

## E6 — Thor CUDA memory and GPUDirect Storage investigation

- [ ] Extend external leases with a producer-readiness condition. GPU completion
  precedes storage submission; prohibit GPU mutation until all storage reads retire.
- [ ] First qualify aligned system-memory allocations written directly by CUDA,
  followed by CUDA-registered host memory. Qualify managed/device allocations
  individually for mapping, pinning and direct-I/O compatibility.
- [ ] Investigate cuFile/GPUDirect Storage as an optional backend. Verify support
  for the actual Jetson Thor software, filesystem and device configuration before
  implementation. Record compatibility-mode fallback separately from direct DMA.
- Gate: on physical Thor, test GPU-generated payload hashes, delayed CUDA events,
  premature reuse prevention, I/O failures and shutdown. Measure payload copies,
  CPU, memory bandwidth and throughput against explicit-copy recording. Report
  allocation types and backend paths actually validated. ARM64 WSL supplies no
  evidence of Thor GPU/storage DMA behavior.

References: [CUDA for Tegra](https://docs.nvidia.com/cuda/cuda-for-tegra-appnote/),
[GPUDirect Storage](https://docs.nvidia.com/gpudirect-storage/overview-guide/).
