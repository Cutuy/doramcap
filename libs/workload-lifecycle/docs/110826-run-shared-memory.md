![Shared memory within a run or with the host](110826-run-shared-memory.svg)

# Shared memory: architecture

Created: 2026-11-08.

Apps in a run share the `/dev/shm` filesystem. The same rules apply to user and
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
Review and launch use the same prepared configuration. Native declarations can
bind host hugetlbfs at custom paths; those sources stay live and host-owned.

systemd preserves native IPC settings. Its run default applies to the filesystem
shared through `/dev/shm`.

## Experiment snapshots

The common host observer captures host shm as-is for existing A/B snapshots,
independently of launch backend and app mounts. Capture paths, ownership,
permissions, sizes and timestamps without reading, copying or hashing contents. Work
scales with object count, independently of byte size. Live observations stay
separate from Review's configuration identity.

Observation returns `Result<MachineState, String>`. Failed captures remain absent;
Console reports their errors separately.

## Cleanup

Release the run-owned filesystem after verified shutdown; leave host memory alone.
Docker uses a run infrastructure container to retain its shared `/dev/shm`; systemd
retains a tmpfs mount. If shutdown is unconfirmed, retain run resources and report
incomplete cleanup.

See the [implementation plan](110826-run-shared-memory-impl.md).
This design replaces [#206](https://github.com/Cutuy/doramcap/pull/206).
