![Shared-memory launch flow](110826-run-shared-memory-impl.svg)

# Shared memory: implementation

Created: 2026-11-08. Implements the [architecture](110826-run-shared-memory.md).
Implementation has not started; the milestones below are pending.

## Changes

Add one boolean field, `host_shared_memory`, to the existing `RunPlan`: false
selects Isolated mode; true selects Host pass-through. No new settings message.
Remove any `host_shms` field and support; none exists in the current source.

| Backend | Main change |
| --- | --- |
| Docker | Attach participating containers to the same run shm and IPC setup, with or without Dora. |
| systemd | Mount the same run-private shm filesystem in participating units. |

Apply accepted native declarations to the launched containers and units. Source
Compose and unit files remain unchanged. Review and launch use the same prepared
configuration; existing backend cleanup releases run-owned mounts and namespaces.

## Native settings

| Native setting | In Isolated mode |
| --- | --- |
| None | Mount run memory at `/dev/shm` |
| Selective host bind | Keep run memory; bind the host source at the declared target and permissions |
| Whole-host shm or Docker `ipc: host` | Fail preparation; identify the app and setting and require global Host mode |
| Explicit private shm | Apply the declaration; reject incompatible IPC or mount settings |

Host mode uses host shm by default; explicit private settings still apply.
Resolve host sources before installing the run's private view, including alternate
targets. Read-only host shm stays live through preparation and launch.

## Snapshots

Extend existing [Docker snapshots](../../workload-docker/src/workflows/recorded_compose/preparation/snapshot.rs)
and [systemd capture](../../workload-systemd/src/preparation.rs) with host shm
identity, size, permissions and timestamps. Use filesystem/IPC metadata operations;
for directory or whole-host access, enumerate exposed objects. Read, copy and hash
no shm payload. Cost follows object count, independently of byte size. Record
capture errors. Keep live observations separate from Review/Start configuration
comparison; ordinary configuration files retain their existing byte capture.

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

- [ ] Wire the single field through existing callers and preparation; remove any remaining `host_shms` support.
- [ ] Resolve native settings through Compose merges and systemd unit/drop-in loading.
- [ ] Capture host shm metadata in existing snapshots while keeping mounts live.

**Validation**

- [ ] User/system apps and pbtxt/native inputs behave consistently; accepted settings appear in real containers and units.
- [ ] Read-only host binds stay live at usual and alternate targets: host writes are visible, app writes are denied.
- [ ] Live writes leave Review valid; changed mount declarations or captured configuration require fresh preparation.
- [ ] A/B snapshots record host shm metadata and capture errors without payload reads; larger shm objects do not increase capture I/O.

## Milestone 3 — Implement both backends

**Work items**

- [ ] Connect shared mounts and namespaces to existing startup, shutdown and cleanup; remove unconditional Dora IPC overrides.

**Validation**

- [ ] Isolated mode shares run memory; selective host binds stay live. Cover read-only/writable binds, files/directories and alternate targets.
- [ ] Whole-host mounts and Docker `ipc: host` fail before launch in Isolated mode and work in Host mode; host/app writes are mutually visible.
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
