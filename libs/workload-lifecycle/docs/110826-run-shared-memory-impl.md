![Shared-memory launch flow](110826-run-shared-memory-impl.svg)

# Shared memory: implementation

Created: 2026-11-08. Implements the [architecture](110826-run-shared-memory.md).
Implementation has not started; the milestones below are pending.

## Where the changes belong

Add one protobuf setting for the global mode. Console and direct callers pass it
through `RunPlan`. Preparation applies explicit native settings first, then fills
in defaults. Review and launch use that same prepared configuration.

| Backend | Main change |
| --- | --- |
| Docker | Share run memory even without Dora. Preserve declared IPC settings and host mounts. |
| systemd | Give the run a shared private shm filesystem. Preserve explicit unit settings. |

Each backend tracks the memory it creates and removes it through its existing
stop/cleanup flow. Live host shm stays outside filesystem overlays.

## When the global mode redirects

Use native mounts and add our run mount only where the app has left room for it.

| Explicit native setting | Final setup for that app |
| --- | --- |
| None | Mount run memory at `/dev/shm` |
| One host segment | Keep run memory; bind the host segment at its declared target with its declared permissions |
| Entire host shm, or Docker `ipc: host` | Skip the conflicting run-shm setup for that container or unit |

Read native settings before generating mounts. Resolve native sources from the
host before installing the run's private view. A native target such as
`/input/camera` must still refer to the original host segment.

Keep host shm live through preparation: record its mount declaration, without
copying or hashing its changing contents. Read-only controls app writes; it does
not change which memory the mount uses. Ordinary configuration files still get
captured.

## Decisions still open

- What keeps run memory alive when an app exits? Prove the approach before adding a helper process or dependency.
- What effective capacity does each backend give a new run pool? Agree the defaults and handling of conflicting native sizes before implementation. No new size setting or numeric default has been approved.

Apps sharing a pool share its capacity. Host binds use host capacity; a native
private filesystem has its own limit. Preserve existing app-failure handling when
allocations fail and report the observed outcome.

## Required evidence

Sharing, mount precedence, permissions, capacity and cleanup must be tested on a
real Docker Engine and a real systemd service manager, using real producer and
consumer processes. Mocks and generated-configuration assertions cannot satisfy
these acceptance checks. Record mount/IPC observations and actual reads/writes;
mark a missing platform check as blocked.

## Milestone 1 — Prove sharing and lifetime

**Work items**

- [ ] Try the smallest native approach on Docker and systemd, with and without Dora.
- [ ] Choose how run memory stays alive and how it is recovered after a crash.
- [ ] Record each backend's effective run capacity and agree the default and native-size conflict rules.

**Validation**

- [ ] Two apps exchange live data; another run and the host's ordinary shm view cannot see it.
- [ ] App exit, failed startup and controller crashes leave resources that can be cleaned up safely.
- [ ] Check POSIX and System V shared memory separately.

## Milestone 2 — Apply settings consistently

**Work items**

- [ ] Wire the two global modes through configuration, `RunPlan` and preparation; reject unknown modes.
- [ ] Keep track of explicit native settings through Compose merges and systemd unit/drop-in loading.
- [ ] Preserve live host shm files through systemd's [read-only file capture](../../workload-systemd/src/preparation.rs), including alternate mount targets.

**Validation**

- [ ] Real launches distinguish missing settings from explicit ones; user/system apps and pbtxt/native inputs behave consistently.
- [ ] A read-only host shm file stays live after preparation and launch, at both its usual path and an alternate target. Host writes are visible; app writes are denied.
- [ ] Live data changes leave Review valid. Changing the mount declaration or captured configuration requires fresh preparation; ordinary configuration capture still works.

## Milestone 3 — Implement both backends

**Work items**

- [ ] Connect the proven sharing approach to app startup, shutdown and cleanup.
- [ ] Remove unconditional Dora IPC overrides and preserve explicit host shm mounts through filesystem preparation.

**Validation**

- [ ] With redirect and no override, real apps use the same run memory; host memory remains separate.
- [ ] With redirect and one native host bind, the selected host segment stays live while ordinary shm still belongs to the run. Cover read-only and writable binds, files/directories, and alternate targets.
- [ ] With redirect and a whole-host mount (or Docker `ipc: host`), the app accesses host memory directly. Files created by host and app after startup are visible to each other.
- [ ] With host mode, apps use host memory; explicit private native shm settings still take priority.
- [ ] At least two apps fill the shared pool with real writes and touched mapped pages. Verify the effective limit, exhaustion failures/app status and cleanup; check host/private overrides use their own backing capacity.
- [ ] Explicit IPC, mount and capacity settings are preserved. Incompatible requirements produce a clear error before launch.
- [ ] Host objects and app writes to them survive cleanup; run-owned memory disappears after verified shutdown. Cover partial startup, cancellation and abandoned runs.

## Milestone 4 — Check complete workloads

**Work items**

- [ ] Add small runnable examples and link them from the docs.
- [ ] Exercise Console, direct callers and Dora on both backends.

**Validation**

- [ ] Focused Rust release/locked checks, Console tests and TypeScript checking pass.
- [ ] Dora exchanges data and finishes recordings in both modes; test native overrides that split apps into different shm groups.
- [ ] Run the [required local CI](../../../AGENTS.md#testing-and-ci) and ARM64 checks at integration. Record native backend and GPU checks separately.

Evidence: document links, milestone structure and design review pass. All runtime
acceptance checks remain pending; no native test results are claimed.
