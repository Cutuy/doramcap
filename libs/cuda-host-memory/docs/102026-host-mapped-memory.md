![Implemented mapping and execution](host-registration.svg)

# Host-mapped CUDA memory — 2026-10-20

## Milestones

- [x] Preserve discussion SVGs before implementation: `design-history/*.svg`
  and `slot-ownership.svg`. These describe proposals and superseded alternatives.
- [x] Add workspace crate using `memmap2` and `cudarc` 0.19.10.
- [x] Implement anonymous `mmap → cudaHostRegister(Mapped) → cudaHostGetDevicePointer`.
- [x] Demonstrate two kernels on an explicit nonblocking, non-default stream.
- [x] Document unsafe CPU/device access and blocking context-wide cleanup in
  [Rust API documentation](../src/lib.rs).
- [x] Add owning stream leases and runtime CPU/GPU access checking (2026-10-21).
- [ ] Add asynchronous retirement, pool reuse and IPC ownership in subsequent work.

## Validation evidence

Local GPU: NVIDIA GeForce RTX 5070 Ti; driver 595.104.02; toolkit 12.9.

- [x] `cargo run -p cuda-host-memory --example non_default_stream`:
  1,048,576 bytes verified after two in-place GPU updates; no payload copies.
- [x] `cargo test -p cuda-host-memory --example non_default_stream -- --ignored --nocapture`:
  2 tests passed. Verified lengths 1, 4095, 4096, 4097 and 1,048,576;
  checked both explicit close and Drop with queued device writes.
- [x] CUDA Compute Sanitizer `memcheck --error-exitcode 1` on the hardware
  test binary (`--ignored --test-threads=1`): 2 passed, **0 errors**.
- [x] `cargo test -p cuda-host-memory --all-targets`: 1 passed, 2 hardware
  tests explicitly ignored by default. Hardware tests were run separately above.
- [x] `cargo clippy -p cuda-host-memory --all-targets --all-features -- -D warnings`:
  passed. Formatting, documentation tests and `git diff --check` also passed.
- ARM CI skipped with user authorization: SSH probe found no `nvidia-smi`,
  `nvcc`, `/usr/local/cuda*/lib64/libnvrtc.so*`, or `/usr/lib/wsl/lib/libcuda.so*`.
  No ARM CUDA behavior is claimed.

## Checked ownership update — 2026-10-21

- CPU `read()` / `write()` guards enforce whole-buffer access exclusion.
- `gpu(stream, Access)` owns a reservation and validates the context before use.
- `enqueue` is the only public unsafe function: kernel bounds, access mode,
  stream confinement and other kernel arguments remain the caller’s contract.
- `wait()` and GPU-lease drop synchronize the stream before releasing access.
  Panic/error paths preserve ownership. Forgotten leases retain storage.
- Completion errors permanently quarantine the allocation and reject new access.
- GPU-lease drop can block; asynchronous reclamation and IPC are future work.

Validation evidence (local RTX 5070 Ti):

- [x] Five hardware tests passed: boundary-sized buffers; CPU/GPU conflict
  rejection and owner lifetime; unwind after launch; error after launch; wrong context.
- [x] Compute Sanitizer memcheck: all five tests passed, **0 errors**.
- [x] Two default unit tests passed; five hardware tests explicitly ignored in
  default runs and executed separately above.
- [x] Updated demo verified 1 MiB with safe CPU guards and two non-default-stream kernels.
- [x] Clippy with `-D warnings`, formatting and diff checks passed.


## Current boundaries

- Leases cover the entire mapping in this process. Concurrent CPU/GPU reads are
  supported; writes exclude every other lease.
- Final registration cleanup synchronizes the context, unregisters, then unmaps.
  Cleanup failure retains storage and context indefinitely.
- Anonymous memory only; cross-process exports, pool reuse and host callbacks
  are not implemented.
- CUDA libraries load at runtime; missing libraries cause cudarc’s loader to panic.

Reference: [NVIDIA mapped host memory documentation](https://docs.nvidia.com/cuda/cuda-programming-guide/02-basics/understanding-memory.html).

## Coverage review — 2026-10-21

Tests moved out of the sequential example into integration suites; examples are
standalone programs. Earlier evidence above describes earlier revisions.

| Contract | Current coverage |
|---|---|
| CPU/GPU read overlap | Bounded GPU-start/progress/release handshake; both compute checksums of the same bytes |
| Borrow conflicts | All 16 CPU/GPU read/write combinations; rejected attempts leave state unchanged; reacquisition succeeds |
| Multiple CPU threads | Barrier-coordinated contention: exactly one writer wins, both readers succeed |
| Multiple streams | Real read kernels on two non-default streams; each reservation retires independently |
| Data visibility | Two ordered write kernels; byte verification at 1, 4095, 4096, 4097 and 1 MiB |
| Ownership | CPU guard outlives handle; GPU lease is sole owner after handle drop |
| Early release | GPU drop waits; CPU/GPU unwind releases correctly after submitted work |
| Submission failure | Injected host-side error after a real kernel launch retains the reservation |
| Invalid inputs | Public constructor rejects invalid lengths; foreign CUDA context rejected |
| Completion failure | Real device trap in isolated child; subsequent CPU/GPU reads and writes return `Poisoned` |
| Rust API misuse | Four compile-fail cases: mutate read guard, escape slice lifetime, clone GPU lease, enqueue without unsafe |

The overlap handshake uses **separate test-only registered control pages** and
aligned system-scope 32-bit atomic loads/stores. Payload bytes remain under read
leases. CPU timeout and a finite GPU watchdog turn missing progress into failure.
The test checks successful handshake completion, so watchdog exit cannot count
as a passing overlap test. See [CUDA atomicity requirements](https://nvidia.github.io/cccl/unstable/libcudacxx/extended_api/memory_model.html).

### Current evidence

- [x] `ownership` integration suite: **11 passed** on the RTX 5070 Ti.
- [x] Isolated completion-failure parent: **1 passed**, verifying its CUDA-fault child.
- [x] Compute Sanitizer memcheck on `ownership`: **0 errors**, all 11 passed.
  The deliberate fault suite runs separately because its trap is expected.
- [x] `concurrent_read` example: GPU progress observed between CPU reads;
  CPU/GPU writers rejected; write reuse succeeded after retirement.
- [x] Default tests: **2 unit tests passed**, hardware tests ignored by default.
- [x] **4 compile-fail doctests passed**; strict Clippy passed.

### Limits

- CUDA registration/unregistration failures and allocation exhaustion are not
  fault-injected. ARM/Thor behavior remains unverified; worker CUDA is unavailable.
- Kernels violating the unsafe enqueue contract (out-of-bounds access, undeclared
  writes, or escaped pointers) are outside runtime borrow checking. No test
  claims these are automatically detected.
- Forgotten leases retain storage by design; no automatic leak recovery is promised.

## Managed allocation — 2026-10-21

- `new()` keeps registered anonymous host memory as its default.
- `new_with_mode(..., AllocationMode::Managed)` uses `cudaMallocManaged` with
  global attachment, zero-initializes bytes and uses `cudaFree` on cleanup.
- CPU/GPU guards, borrow reservations, stream completion and quarantine are shared.
- Requires `managedMemory` and `concurrentManagedAccess`; unsupported devices return
  `ManagedMemoryUnsupported`. No silent fallback or reduced concurrency guarantee.
  CUDA controls managed page migration. See [NVIDIA concurrency requirements](https://docs.nvidia.com/cuda/cuda-programming-guide/04-special-topics/unified-memory.html).

### Shared coverage

The 11 ownership scenarios take an allocation mode. A macro produces `registered::`
and `managed::` tests from that single scenario list. Registered scenarios exercise
the default constructor. The fault test runs the same child body in a fresh process
for each mode. Three default unit tests include all managed-capability combinations.
Both demos accept `--managed`.

- [x] All **22 ownership cases** passed: 11 scenarios × 2 allocation modes.
- [x] Isolated completion-failure test passed for both modes in separate children.
- [x] Compute Sanitizer memcheck: all 22 cases passed, **0 errors**.
- [x] Both demos passed with `--managed`, including actual CPU/GPU read overlap.
- [x] Three default unit tests and four compile-fail doctests passed.
- [x] Strict Clippy, Rustdoc, formatting and diff checks passed.

Validation used the local RTX 5070 Ti. ARM/Thor remains unverified; the ARM
worker has no detected CUDA installation, as previously authorized to skip.
