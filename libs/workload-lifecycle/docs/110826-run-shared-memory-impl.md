![Shared-memory launch flow](110826-run-shared-memory-impl.svg)

# Shared memory: implementation

Created: 2026-11-08. Implements the [architecture](110826-run-shared-memory.md).
Implemented in #237 (runtime) and #236 (host observation).

## Changes

Add `HostShmAccess host_shm_access` to the existing `RunPlan`, with enum values
`HOST_SHM_ACCESS_UNSPECIFIED = 0`, `HOST_SHM_ACCESS_PASSTHROUGH = 1` and `HOST_SHM_ACCESS_BLOCKED = 2`.
Reject zero and unknown values during preparation. No new settings message.
Remove any `host_shms` field and support; none exists in the current source.

| Backend | Main change |
| --- | --- |
| Docker | Start the configured `doramcap-shm` system app first, join default IPC participants, stop it last. |
| systemd | Bind run tmpfs at `/dev/shm` for generated and native units without an explicit private mount. |

Resolve host sources before installing the run's private view. Existing backend
cleanup releases the run-owned filesystem. Native systemd mounts are checked
through manager properties; native IPC settings remain unchanged.

Extend the shared [host observer](../../resources/system-config/src/lib.rs) and
[`MachineState`](../../resources/system-config/proto/system_config.proto) with
`repeated FileMetadata host_shm`: path, mode, UID/GID, size, and modification/change
times as signed Unix nanoseconds. Return observation errors through `Result` and
propagate them separately to Console. Keep ordinary configuration byte capture intact.

## Required evidence

Use real Docker Engine containers, systemd units and producer/consumer processes.
Record mount/IPC observations and actual reads/writes. Mocks and generated-config
checks cannot satisfy native acceptance; mark unavailable platform checks blocked.

## Milestone 1 — Prove sharing and lifetime

**Work items**

- [x] Try the smallest native approach on Docker and systemd, with and without Dora.

**Validation**

- [x] Two apps exchange live data; other runs and the host's ordinary shm view cannot see it.
- [x] Verify kernel object lifetime across app exit, and cleanup after failed startup or controller crashes.

## Milestone 2 — Apply settings consistently

**Work items**

- [x] Wire the enum field through existing callers and preparation; remove any remaining `host_shms` support.
- [x] Resolve native settings through Compose merges and systemd unit/drop-in loading.
- [x] Add host shm observation to common machine capture and expose capture errors in the UI.

**Validation**

- [x] Unset/unknown modes fail preparation; both supported values work for user/system apps and pbtxt/native inputs.
- [x] Read-only host binds stay live at usual and alternate targets: host writes are visible, app writes are denied.
- [x] Live writes leave Review valid; changed mount declarations or captured configuration require fresh preparation.
- [x] The same host state yields the same shm metadata regardless of backend; larger objects do not increase capture I/O.
- [x] Capture failure shows an error in the UI and leaves the machine capture absent.

## Milestone 3 — Implement both backends

**Work items**

- [x] Connect shared mounts to existing startup, shutdown and cleanup; remove unconditional Dora IPC overrides.

**Validation**

- [x] BLOCKED shares run memory; selective host binds stay live. Cover read-only/writable binds, files/directories and alternate targets.
- [x] Whole-host mounts and Docker `ipc: host` fail before launch with BLOCKED and work with PASSTHROUGH; host/app writes are mutually visible.
- [x] Explicit private settings apply; incompatible IPC or mounts fail with a clear error.
- [x] Host objects survive cleanup; run resources disappear after verified shutdown. Cover partial startup, cancellation and abandoned runs.

## Milestone 4 — Check complete workloads

**Work items**

- [x] Add small runnable examples; exercise Console, direct callers and Dora on both backends.

**Validation**

- [x] Focused Rust release/locked checks, Console tests and TypeScript checking pass.
- [x] Dora exchanges data and finishes recordings in both modes.
- [ ] Qualify declared hugetlbfs mounts with real Docker/systemd workloads.
- [ ] Qualify GPU/CUDA workloads.
- [x] Run [required local CI](../../../AGENTS.md#testing-and-ci) and ARM64 checks at integration; record native backend and GPU checks separately.

Current follow-up: enum values use the `HOST_SHM_ACCESS_` prefix. Native mount
declarations remain authoritative; hugetlbfs sources use filesystem metadata.
Standalone Rust/TypeScript fixtures replace embedded Python. Path checks and the
metadata-only hugepage probe pass; native tests compile. Native mount tests, full
CI and ARM64 are not rerun for this follow-up.

Earlier local CI and ARM64 jobs 617/619 passed before this simplification, with
real Docker/systemd, Dora and Console recording checks. The earlier alias-fixture
mount incident was repaired; host-mount tests remain disabled on this checkout.
