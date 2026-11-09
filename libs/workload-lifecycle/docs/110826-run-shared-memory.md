![Shared memory within a run or with the host](110826-run-shared-memory.svg)

# Shared memory: architecture

Created: 2026-11-08.

Apps in a run share memory with each other. The same rules apply to user and
system apps on Docker and systemd, using pbtxt or native files.

## Host shm access

Select one of two enum values explicitly. The unset/default value and unknown
values are unsupported and fail preparation.

| Mode | What apps use | What happens after the run |
| --- | --- | --- |
| **BLOCKED** | Shared memory belonging to this run | Release it after the apps stop |
| **PASSTHROUGH** | The host's shared memory, with normal access permissions | Leave it alone |

## Native settings

| Native setting | With BLOCKED |
| --- | --- |
| None | Mount run memory at `/dev/shm` |
| Selective host bind | Keep run memory; bind the host source at the declared target and permissions |
| Whole-host shm or Docker `ipc: host` | Fail preparation; identify the app and setting and require PASSTHROUGH |
| Explicit private shm | Apply the declaration; reject incompatible IPC or mount settings |

PASSTHROUGH uses host shm; explicit private settings still apply. Selective host
binds stay live, including read-only binds and alternate targets. Apply accepted
declarations to launched apps; source Compose and unit files remain unchanged.
Review and launch use the same prepared configuration.

BLOCKED rejects native systemd units whose effective IPC scope cannot be
established before launch. Generated units support the run default.

## Experiment snapshots

The common host observer captures host shm as-is for existing A/B snapshots,
independently of launch backend and app mounts. Capture object identity, size,
permissions and timestamps without reading, copying or hashing contents. Work
scales with object count, independently of byte size. Live observations stay
separate from Review's configuration identity.

If capture fails, report the error in the UI and leave the host shm field
empty/absent.

## Cleanup

The kernel manages shm lifetime. Release run-owned mounts and namespaces after
verified shutdown; leave host memory alone. Docker uses a run infrastructure
container to retain its namespace; systemd retains kernel mounts without a helper
process. If shutdown is unconfirmed, retain
run resources and report incomplete cleanup.

See the [implementation plan](110826-run-shared-memory-impl.md).
This design replaces [#206](https://github.com/Cutuy/doramcap/pull/206).
