![Shared memory within a run or with the host](110826-run-shared-memory.svg)

# Shared memory: architecture

Created: 2026-11-08. Sharing behavior agreed; implementation and default capacity pending.

Apps in a run should share memory with each other. By default, that memory stays
separate from the host and other runs. When the apps stop, we remove it.

The same rules apply to user apps and system apps, whether they run in Docker or
systemd and whether their settings come from pbtxt or native files.

## Two global modes

| Mode | What apps use | What happens after the run |
| --- | --- | --- |
| **Isolated — default** | Shared memory belonging to this run | We remove it after the apps stop |
| **Host** | The host's shared memory, with normal access permissions | We leave it alone |

## Explicit app settings take priority

An app's explicit native settings override the global mode for that app and path.
We supply defaults only where the app has left the relevant setting unspecified.

For example, an app can mount one host segment read-only while keeping the rest
of its shared memory private to the run. It can also request its own private
memory or the entire host shm. Those choices may change which apps can share data.
A host segment stays live even when mounted read-only: host updates remain visible.

Selective access belongs in native mounts. We do not need a global read-only
mode or a separate list of host shm names.

## Shared capacity

Apps using the same run memory share its capacity. Direct host mounts use the
host's capacity. The default run-pool size still needs to be agreed.

## Cleanup follows ownership

We remove memory that belongs to the run. We preserve host memory and externally
owned mounts. If we cannot confirm that the apps have stopped, we keep the run's
resources and report that cleanup is incomplete.

See the [implementation design and work plan](110826-run-shared-memory-implementation.md)
for how to build and test this. This design replaces [#206](https://github.com/Cutuy/doramcap/pull/206).
