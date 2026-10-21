![Architecture](fixed-slot-ipc.svg)

# Fixed-slot CUDA host-memory IPC — 2026-10-21

## Milestones

- [x] Register owned file mappings without changing their contents.
- [x] Preallocate bounded tmpfs slots; reject managed IPC.
- [x] Exchange cached Ready/Done **event** handles through Dora metadata.
- [x] Publish pending GPU writes; consumer waits in its chosen stream.
- [x] Require all generation returns and event completions before reuse.
- [x] Build and run real producer + multiple consumer processes.

Implementation: `src/lib.rs`, `src/ledger.rs`, `src/event.rs`, and
`apps/cuda-shm-demo/src/main.rs`. No CUDA IPC memory handles are used.

## Ownership and ordering

| Layer | Ownership / release condition |
|---|---|
| Local Rust | CPU guards and stream-bound GPU leases enforce runtime reader/writer exclusion. |
| Cross-process | Producer reserves every named reader before publishing a generation. |
| Ready | Producer records after writes; consumers queue a wait before GPU reads. |
| Done | Each reader records after its final GPU read, then sends Returned. |
| Reuse | All Returned messages plus completed Ready and all Done events. |
| Shutdown | Producer closes Done imports; consumers close Ready imports and Done exports; producer closes Ready exports. |

`Arc` counts stay process-local. Dora transport acknowledgments do not retire
payloads. A Returned message means the Done record was submitted; it does not
mean GPU completion. The producer adapter queries events to retire slots.
No CUDA host callback is installed.

A slot's generation is carried in Frame/Returned, not embedded in CUDA events.
An event is re-recorded only after its previous generation retires. A consumer
cannot return twice, skip a generation, or return an older generation. An
unpublished reservation may be canceled without exposing its generation.

## Boundaries

- Payloads: ordinary RAM, one private `/dev/shm` ring file. All processes
  map, mlock and register the whole ring independently. Pages are materialized and locked before CUDA registration.
- Fixed slots/capacity/consumer membership. No payload allocation after setup;
  control-message and bookkeeping allocations still occur per frame.
- Same Linux boot and GPU UUID required. No remote payload fallback, dynamic
  joins, managed IPC, VRAM allocation, PCIe BAR mapping, or DMA fence backend.
- IPC event support is device/platform-dependent; CUDA errors are returned.
  Local validation used an RTX 5070 Ti; Thor is not validated here.
- Safe access methods rely on unsafe construction and message-acceptance trust contracts and unsafe
  kernel-launch contracts. External processes can violate protocol or truncate
  files; Rust cannot prevent that.
- Lost messages or crashed/stalled peers retain slots. No timeout-based reuse.
  Restart the flow to recover. Abnormal exporter teardown retains event resources
  until process exit; kernel errors quarantine local leases.
- Reclamation uses event queries. Local lease destruction synchronizes its stream;
  unrelated later work on that stream can delay retirement. Dedicated streams
  reduce that coupling.

## Validation

- [x] Build crate and demo: `cargo build -p cuda-shm-demo`.
- [x] Real Dora 1.1.0: 3 consumers, 12 frames, 2 slots, generations through 6.
  All consumers GPU-verified every frame; 11 publications were pending; 12 reclaimed.
- [x] Slow work/backpressure: 137 full-pool attempts and 252 completion-blocked checks.
- [x] Stalled reader: 2 published, 0 reclaimed; timeout without overwrite.
- [x] Duplicate return: producer rejects `duplicate return`.
- [x] Compute Sanitizer: 2 consumers, 6 frames; all three processes report 0 errors.
- [x] Seven adapter unit tests; mapping regression passes on `/dev/shm`.
- [x] Existing 22 GPU ownership cases pass (registered and managed local memory).
- [x] Strict Clippy, formatting, four compile-fail ownership doctests pass.
- [x] Single-slot stress: 3 consumers, 8 frames, generation 8; every publication
  pending, 8 reclaimed, 175 backpressure attempts.
- [x] Cross-page payloads: 4,097-byte and 1 MiB frames; two consumers each verify
  four frames through two slots (two generations).
- [x] Registered-memory completion-failure subprocess confirms quarantine.
- [x] Rustdoc builds with warnings denied; architecture SVG rendered and inspected.
- [x] ARM64 full suite: final snapshot passed as Pueue job 107; jobs 104/105
  also passed. First final-snapshot attempt (106) hit the existing storage test
  `rejection_combining_and_rotation_are_visible` at `observability.rs:211`;
  identical-source retry passed. CUDA execution skipped: worker has neither
  `nvcc` nor `nvidia-smi`.


## Whole-ring backing and slot borrows

- [x] One tmpfs-verified file; one mapping, eager mlock and CUDA registration per process.
- [x] Independent disjoint slot leases retain an Arc to the shared registration.
  Splitting consumes an unaliased owner; existing clones/guards reject splitting.
- [x] Protocol v2 carries payload capacity and 64-byte-aligned slot stride.
- [x] `/proc/self/smaps` confirms the entire ring is locked; zero RLIMIT_MEMLOCK
  in an isolated child rejects setup. Locks remain until backing unmap.
- [x] Two new region tests plus all 22 existing GPU ownership cases pass.
- [x] Three mapping/locking tests and 10 CPU unit tests pass; strict Clippy passes.
- [x] Dora: 3 consumers, 12 frames, 4,097-byte capacity and aligned slot strides;
  all frames verified, all 12 published pending and reclaimed. Stalled reader
  holds both slots with zero reclamation. A 2-consumer, 1 MiB/slot run also
  verifies four frames and two generations through one locked 2 MiB ring.
- [x] Compute Sanitizer: 2 consumers, 6 frames, zero errors in all 3 processes.
- [x] ARM64 full worker suite passed, Pueue job 108. CUDA execution remains
  local because the ARM64 worker has no CUDA installation.

Event exchange remains unchanged: create with `cudaEventInterprocess` and
`cudaEventDisableTiming`; `cudaIpcGetEventHandle` exports 64 opaque bytes through
Manifest (Ready) or Attached (Done). Peers use `cudaIpcOpenEventHandle`, then
`cudaStreamWaitEvent` or `cudaEventQuery`. Shutdown closes imports before exports.
These are event handles; payload sharing uses the locked tmpfs mapping.
