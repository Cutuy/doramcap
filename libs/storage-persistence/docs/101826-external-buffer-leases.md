# External-memory persistence

![Three input paths and the shared writer lifecycle](architecture/three-path-writer-lifecycle.svg)

Status: planned · revised 2026-10-19. Builds on the implemented [managed pool](101926-growable-buffer-pool.md).
Two distinct APIs share external ownership and I/O: large payloads need storage framing;
prepared blocks supply complete file records and bypass combining at any aligned size.
[Input contracts](101926-memory-ingress-cases.md) · [Ownership lifetimes](architecture/external-memory-lifecycle.svg).

## E1 — Shared external-region and read-lease support

- [ ] Evaluate [memmap2](https://docs.rs/memmap2/latest/memmap2/) for the adapter/tests; keep storage independent of its allocator.
- [ ] Accept caller-mapped `MAP_SHARED` regions from memfd and `/dev/shm`.
- [ ] Keep the API to a mapped region with a validity guard and a ready byte-range lease with a reuse guard; allow one provider object to supply both.
- [ ] Require mapping/backing identity through registration retirement and immutable bytes through write retirement; document unsafe construction for caller-enforced contracts.
- [ ] Register lazily using G2's manager; retain registrations across messages and rotation. Return rejected leases to callers.
- [ ] Validation: producer handoff, repeated region reuse, mapping identity/generation and guard-drop counts are correct.
- [ ] Validation: registration and admission failures preserve existing resources and caller ownership.
- [ ] Generalize pending writes to retain managed or external leases; reuse the current file-offset allocator, SQEs, splitting, retries and CQE retirement.
- [ ] Keep mapping-registration guards separate from byte-reuse leases; track region identity independently of recycled pointer values.
- [ ] Validation: append an aligned mmap range directly; verify exact readback, one registration across reuse, and release only after the final CQE.
- [ ] Validation: a retained registration permits completed blocks to recycle; explicit retirement releases mapping guards safely.

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

- [ ] Add `append_prepared_records(block)` independently of E2's payload-size policy; accept any supported aligned block size.
- [ ] Provide a framing helper for producer-reserved MCAP header/padding space and a sealed prepared-block type.
- [ ] Admit existing bytes unchanged; bypass combining, serialization and framing writes after sealing.
- [ ] Preserve channel/schema compatibility, previous combining-batch order, rotation and finalization; return rejected blocks intact.
- [ ] Integrate tracing's slab owner and shared payload readers; the last reader/I/O lease recycles the slab.
- [ ] Validation: 4-KiB and 64-KiB prepared blocks reach direct I/O with zero storage copies and exact independent MCAP readback.
- [ ] Validation: hold readers beyond CQEs and CQEs beyond readers; recycling waits for both while registration persists.
- [ ] Validation: mixed managed/payload/prepared appends preserve file order under queue pressure, cancellation and rotation.

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
