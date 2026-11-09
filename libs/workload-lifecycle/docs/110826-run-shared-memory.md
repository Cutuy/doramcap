![Shared memory within a run or with the host](110826-run-shared-memory.svg)

# Shared memory: architecture

Created: 2026-11-08. Implementation pending.

Apps in a run share memory with each other. By default, that memory stays separate
from the host and other runs. The same rules apply to user and system apps on
Docker and systemd, using pbtxt or native files.

## Two global modes

| Mode | What apps use | What happens after the run |
| --- | --- | --- |
| **Isolated — default** | Shared memory belonging to this run | Release it after the apps stop |
| **Host** | The host's shared memory, with normal access permissions | Leave it alone |

## Native settings

Apply native IPC and mount declarations to the launched apps. A selective host
bind stays live, including read-only binds; explicit private settings may separate
sharing groups. Whole-host mounts and Docker `ipc: host` require global **Host**
mode. Reject conflicting settings before launch with an error identifying the app
and setting.

## Experiment snapshots

Extend existing A/B snapshots with host shm metadata. Capture object identity,
size, permissions and timestamps without reading or hashing contents. Work scales
with object count, independently of byte size. Live observations stay separate
from Review's configuration identity.

## Cleanup

The kernel manages shm lifetime. Release run-owned mounts and namespaces after
verified shutdown; leave host memory alone. If shutdown is unconfirmed, retain
run resources and report incomplete cleanup.

See the [implementation plan](110826-run-shared-memory-impl.md).
This design replaces [#206](https://github.com/Cutuy/doramcap/pull/206).
