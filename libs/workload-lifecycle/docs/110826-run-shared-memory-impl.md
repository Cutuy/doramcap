![Shared-memory launch flow](110826-run-shared-memory-impl.svg)

# Shared memory: implementation

Created: 2026-11-08. Implements the [architecture](110826-run-shared-memory.md).
Host observation is implemented in #236; runtime milestones remain pending.

## Changes

Add `HostShmAccess host_shm_access` to the existing `RunPlan`, with enum values
`HOST_SHM_ACCESS_UNSPECIFIED = 0`, `PASSTHROUGH = 1` and `BLOCKED = 2`.
Reject zero and unknown values during preparation. No new settings message.
Remove any `host_shms` field and support; none exists in the current source.

| Backend | Main change |
| --- | --- |
| Docker | Attach participating containers to the same run shm and IPC setup, with or without Dora. |
| systemd | Mount the same run-private shm filesystem in participating units. |

Resolve host sources before installing the run's private view. Existing backend
cleanup releases run-owned mounts and namespaces.

Extend the shared [host observer](../../resources/system-config/src/lib.rs) and
[`MachineState`](../../resources/system-config/proto/system_config.proto) with host
shm metadata and capture errors. Use filesystem/IPC metadata operations and
propagate errors to Console's UI. Keep ordinary configuration byte capture intact.

## Required evidence

Use real Docker Engine containers, systemd units and producer/consumer processes.
Record mount/IPC observations and actual reads/writes. Mocks and generated-config
checks cannot satisfy native acceptance; mark unavailable platform checks blocked.

## Milestone 1 — Prove sharing and lifetime

**Work items**

- [ ] Try the smallest native approach on Docker and systemd, with and without Dora.

**Validation**

- [ ] Two apps exchange live data; other runs and the host's ordinary shm view cannot see it. Check POSIX and System V separately.
- [ ] Verify kernel object lifetime across app exit, and cleanup after failed startup or controller crashes.

## Milestone 2 — Apply settings consistently

**Work items**

- [ ] Wire the enum field through existing callers and preparation; remove any remaining `host_shms` support.
- [ ] Resolve native settings through Compose merges and systemd unit/drop-in loading.
- [x] Add host shm observation to common machine capture and expose capture errors in the UI.

**Validation**

- [ ] Unset/unknown modes fail preparation; both supported values work for user/system apps and pbtxt/native inputs.
- [ ] Read-only host binds stay live at usual and alternate targets: host writes are visible, app writes are denied.
- [ ] Live writes leave Review valid; changed mount declarations or captured configuration require fresh preparation.
- [ ] The same host state yields the same shm metadata regardless of backend; larger objects do not increase capture I/O.
- [x] Capture failure shows an error in the UI and leaves the host shm field absent.

Host observation (#236): 8 release tests, Clippy, native POSIX/SysV and bind-cycle
checks, 116 server tests and 87 UI tests pass. Syscall tracing confirms no payload
reads/maps at 4 KiB and 1 TiB; real addon capture preserves reviewed configuration.
Full local CI and ARM64 remain pending; one opt-in installer test was skipped.

## Milestone 3 — Implement both backends

**Work items**

- [ ] Connect shared mounts and namespaces to existing startup, shutdown and cleanup; remove unconditional Dora IPC overrides.

**Validation**

- [ ] BLOCKED shares run memory; selective host binds stay live. Cover read-only/writable binds, files/directories and alternate targets.
- [ ] Whole-host mounts and Docker `ipc: host` fail before launch with BLOCKED and work with PASSTHROUGH; host/app writes are mutually visible.
- [ ] Explicit private settings apply; incompatible IPC or mounts fail with a clear error.
- [ ] Host objects survive cleanup; run resources disappear after verified shutdown. Cover partial startup, cancellation and abandoned runs.

## Milestone 4 — Check complete workloads

**Work items**

- [ ] Add small runnable examples; exercise Console, direct callers and Dora on both backends.

**Validation**

- [ ] Focused Rust release/locked checks, Console tests and TypeScript checking pass.
- [ ] Dora exchanges data and finishes recordings in both modes, including native settings that split sharing groups.
- [ ] Run [required local CI](../../../AGENTS.md#testing-and-ci) and ARM64 checks at integration; record native backend and GPU checks separately.

Documentation checks are recorded in PR #234. Runtime acceptance remains pending.
