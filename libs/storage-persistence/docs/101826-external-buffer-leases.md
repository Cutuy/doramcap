# External-memory persistence

![Three input paths and the shared writer lifecycle](architecture/three-path-writer-lifecycle.svg)

Status: intra-process slabs qualified; mmap qualified on x86_64/ARM64; transport adapters planned · revised 2026-10-19. Builds on the implemented [managed pool](101926-growable-buffer-pool.md).
Two distinct APIs share external ownership and I/O: large payloads need storage framing;
prepared blocks supply complete file records and bypass combining at any aligned size.
[Input contracts](101926-memory-ingress-cases.md) · [Ownership lifetimes](architecture/external-memory-lifecycle.svg).

## E0 — Intra-process sealed slabs first

![Rust slab ownership and writer retention](architecture/rust-slab-writer-lifecycle.svg)

- [x] Provide bounded producer-owned `SlabPool`, exclusive `WritableSlab` and cloneable `SealedSlab`; consume writable access when sealing.
- [x] Retain `Arc<SlabAllocation>` independently in registration entries; final `Arc<SlabLeaseInner>` release recycles the whole slab.
- [x] Extend pending writes to retain managed or sealed sources through splitting, retry, cancellation and terminal completion.
- [x] Cache registrations per writer by allocation identity; retain resolved segments in pending writes. Pre-register pools before capture and retire explicitly.
- [x] Track actual buffer-table update attempts; slab reuse and file rotation preserve registrations. Keep handles private to their writer and prohibit retirement while requests remain.
- [x] Use the existing aligned allocator and standard-library ownership. `bytes` was evaluated for owned views; `memmap2` remains an E1 adapter candidate. Add no dependency for E0.
- [x] Validation: cross-thread handoff, compile-fail exclusive borrow, reader-before-I/O and I/O-before-reader release, and registration guard lifetime.
- [x] Validation: 1,000 warmed reuse cycles produce zero registration updates; verify exact readback and explicit retirement/re-registration.
- [x] Validation: rejection, partial registration rollback, short/error completions, cancellation and submission failure preserve ownership.
- [x] Validation: default/all-feature tests, Clippy and ARM64 worker result recorded below.

First release uses one `Arc` allocation per seal and a mutex-protected free list.
The storage path submits the entire sealed range; payload-only views and tracing
sink migration remain follow-ups. Each slab allocation is fixed-size; create and
pre-register another pool to add capacity. Mapped ranges use E1's separate registration and reuse guards.

## E1 — Mapped regions and read leases

![Shared mapping ownership and adapter boundaries](architecture/mmap-slab-ownership.svg)

- [x] Adopt `memmap2`: `MmapRegion` owns mapping validity; `WritableRange → ReadLease` controls disjoint bytes.
- [x] Add unsafe provider import/publication with a separate reuse token and generation; retain mapping guards through registration retirement.
- [x] Cache registration by region identity; handle nonzero offsets and segment boundaries; return rejected leases intact.
- [x] Frame owned mapped ranges with `MappedRecordsBuilder → PreparedMappedBlock`; preserve writer binding and rotation.
- [x] Use pinned Zenoh POSIX SHM blocks as the test fixture. Its buffer owner retains the allocation until registration retirement.
- [x] Gate: `MAP_PRIVATE` and `/dev/shm` shared mappings pass x86_64 and ARM64 qualification.
- [x] Gate: split writes, stable registration counts, readers, rejection, rollback and cancellation pass on both architectures.
- [x] Gate: separate-process publication and Zenoh provider guards pass on both architectures.
- [ ] Follow-up: qualify memfd-backed mappings explicitly.
- [ ] Follow-up: production ring/transport adapters provide page-aligned blocks and publication/reuse synchronization.

The core checks range overlap and I/O alignment. Ring headers and synchronization
belong to the provider. The Zenoh fixture retains immutable allocations; independent
transport-block recycling under a retained segment registration remains E3 work.

## E2 — Large external payload API

- [ ] Validate source address, transfer length and destination offset; discover alignment where supported and validate configured fallback requirements.
- [ ] Add `append_external_payload(message, lease)`; copy inputs ≤256 KiB into managed combining storage and release their reuse guards after copying.
- [ ] Assemble managed framing spans around borrowed payloads >256 KiB; preflight all spans and their file range before acceptance.
- [ ] Validation: small-copy and threshold-boundary paths reproduce exact payloads without leaked guards.
- [ ] Persist serialized messages or adapter-provided Protobuf prefixes/blobs/suffixes with decoding metadata.
- [ ] Reject layouts requiring payload edge copies, full copies, buffered fallback or serialization staging above 256 KiB.
- [ ] Bound registration entries/bytes and pending requests; split supported large ranges without copying payloads.
- [ ] Release reuse guards after all writes/retries retire; retire registrations before releasing region guards.
- [ ] Validation: independently decode aligned and rejected-misaligned cases, multiple spans and 100-MiB payloads.
- [ ] Validation: delayed/out-of-order completions, short writes, cancellation, failure and shutdown never permit premature reuse.
- [ ] Validation: large-payload copy count is zero; repeated uses retain one registration per registered subrange.
- [ ] Validation: explicit region retirement releases pins/guards safely; written completion and durability remain distinct.

## E2P — Prepared external block API

- [x] Add `append_prepared_records(block)` independently of E2's payload-size policy; accept any supported aligned block size.
- [x] Provide a framing helper for producer-reserved MCAP header/padding space and a sealed prepared-block type.
- [x] Admit existing bytes unchanged; bypass combining, serialization and framing writes after sealing.
- [x] Preserve channel/schema compatibility, per-piece contiguous ranges, rotation and finalization; return rejected blocks intact.
- [ ] Follow-up: integrate tracing's slab owner and optional payload views; the last reader/I/O lease recycles the slab.
- [x] Validation: 4-KiB and 64-KiB prepared blocks reach direct I/O with zero storage copies and exact independent MCAP readback.
- [x] Validation: hold readers beyond CQEs and CQEs beyond readers; recycling waits for both while registration persists.
- [ ] Validation: mixed managed/payload/prepared appends preserve per-piece contiguity under queue pressure, cancellation and rotation.


## E3 — Dora/Zenoh integration

- [ ] Retain the actual received payload owner; separate mapping lifetime from message reuse where necessary.
- [ ] Identify required adapter/upstream APIs if the received owner cannot expose those lifetimes separately.
- [ ] Account for transport materialization separately from storage copies.
- [ ] Validation: completed messages return to the producer pool while region registration remains valid.
- [ ] Validation: actual Dora transport passes reuse stress, sender exit, receiver shutdown and slow-storage tests with exact hashes.

## E4 — Mixed-workload qualification

- [ ] Run each E1, E2, E2P and E3 gate on x86 and ARM64; verify Pueue results and applicable separate Dora end-to-end tests.
- [ ] Benchmark [three paths](101926-growable-buffer-pool.md#benchmark-paths): plain `fwrite`, direct from already-registered managed memory, and direct from external memory.
- [ ] Set budgets and qualify small combining, large managed reservations and external direct writes together; separate cold registration cost from warmed writes.
- [ ] Validation: every finalized rotated file decodes; pressure and failures preserve accounting and guard reclamation.
- [ ] Validation: record throughput, CPU, p99 latency, allocations, copy/borrow bytes and registration events against budgets.
- [ ] Validation: attach commands, commit, filesystem/platform and worker job; qualify end-to-end DMA on target hardware.

## E5 — Future device providers

- [ ] Qualify driver-owned host RAM for pinning, readiness and reuse.
- [ ] Investigate PCIe BAR/P2P/DMA-BUF backends with driver removal/invalidation protocols.
- [ ] Qualify CUDA-accessible host RAM and GPU readiness/visibility synchronization.
- [ ] Investigate CUDA IPC-imported memory with import/export guards and a cuFile/GPUDirect backend.
- [ ] Validation: backend selection follows backing-memory capabilities; an mmap address or `/dev/shm` name alone never establishes eligibility.
- [ ] Validation: physical Thor tests cover events, reuse, failures and shutdown; large-payload CPU/GPU bounce-buffer paths are rejected.

References: [page pinning](https://docs.kernel.org/core-api/pin_user_pages.html),
[PCI P2P](https://docs.kernel.org/driver-api/pci/p2pdma.html),
[GPUDirect Storage](https://docs.nvidia.com/gpudirect-storage/overview-guide/).

## E0 / E2P validation evidence

- [x] x86_64 Linux, local ext filesystem: default and all-feature storage suites each passed 30 runtime tests and one compile-fail doctest. The existing opt-in 100-MiB registered test remains ignored in these runs.
- [x] `cargo clippy -p storage-persistence --all-targets --all-features --no-deps -- -D warnings`, `cargo fmt --all --check`, and `git diff --check` passed.
- [x] ARM64 WSL2: serial Pueue job **49**, result **Success**, full log inspected. Workspace default/all-feature tests, native cgroup/allocation tests, existing Dora comparisons and storage-demo validation passed.
- [x] New gates: direct 4-KiB/64-KiB MCAP readback, cross-thread builders/readers, mixed managed/prepared ordering, rotation, intact rejection, registration rollback and guard lifetime, cancellation, injected short/error completion, and 1,000 reuse cycles with zero new table updates.

Commands: `cargo test -p storage-persistence`,
`cargo test -p storage-persistence --all-features`,
`python3 scripts/ci/submit.py --source /home/cutuy/doramcap/.git/agent-pool/worktrees/1`,
`ssh doramcap-ci@100.64.0.20 pueue log --full 49`.

Source: base commit `5aa714c3b8c435545095ae695a0b3f58bfa987a0` plus the dirty
snapshot at `/home/doramcap-ci/jobs/c7ef57f1308f47ce973876cc80447ae2`.
`SOURCE.json` records per-file hashes; subsequent local edits update documentation.
Tests live in `tests/sealed_slabs.rs` and the `slabs` / `writer` unit tests. The prepared builder binds records to the
originating writer's schema/channel identity; that identity survives rotation.
Blocks from another writer are rejected intact. Actual target-device DMA and
allocation-free sealing remain unqualified.

## E1 validation evidence

- [x] x86_64 Linux 7.0: default/all-feature storage tests, strict Clippy, formatting and diff checks passed; `/tmp` ext filesystem and `/dev/shm` tmpfs.
- [x] ARM64 WSL2 Linux 6.18: mmap tests passed in default/all-feature runs, including Zenoh, cross-process publication and failure injection.
- [x] ARM64 complete worker suite: Pueue **55**, **Success**; full log inspected. Native accounting, allocation checks, Dora comparisons and storage-demo validation passed.
- [x] Borrow-safety compile-fail test; exact bytes and independent MCAP decoding; unchanged registration counts across repeated ranges; reader and owner drop counts.

Tests: `tests/mapped.rs`, `writer::tests::mapped_segment_offsets_rollback_and_cancellation`.
Commands: `cargo test -p storage-persistence [--all-features]`,
`cargo clippy -p storage-persistence --all-targets --all-features -- -D warnings`.
Worker: `ssh doramcap-ci@100.64.0.20 pueue log --full 55`.
Snapshot: `/home/doramcap-ci/jobs/a19d9b392b314b2a980a02ffa227cfd7`;
`SOURCE.json` records source hashes against base `5aa714c3b8c435545095ae695a0b3f58bfa987a0`.
Subsequent local edits change comments/documentation only. The existing opt-in
100-MiB registered-I/O test remains ignored; physical-device DMA is unqualified.

## Independent write-path validation

- [x] Large managed, sealed-slab and mapped writes use one available pending slot while a small batch remains buffered; exact position-dependent payloads decode after split writes.
- [x] x86_64 default/all-feature tests, strict Clippy, formatting and diff checks passed.
- [x] ARM64 Pueue **59: Success**; full log inspected, including workspace tests and native/application checks. Snapshot: `/home/doramcap-ci/jobs/6c629ad5380846409c5389af4fbcf355`.

Regression: `tests/sealed_slabs.rs::standalone_paths_do_not_flush_or_reserve_capacity_for_combining`.
Each piece occupies contiguous offsets; cross-piece placement and completion order are unspecified.
