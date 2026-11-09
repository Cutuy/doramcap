![Shared memory within a run or with the host](110826-run-shared-memory.svg)

# Shared memory: architecture

Created: 2026-11-08. Updated from PR #234 review; implementation pending.

Apps in a run should share memory with each other. By default, that memory stays
separate from the host and other runs. When the apps stop, we remove it.

The same rules apply to user apps and system apps, whether they run in Docker or
systemd and whether their settings come from pbtxt or native files.

## Two global modes

| Mode | What apps use | What happens after the run |
| --- | --- | --- |
| **Isolated — default** | Shared memory belonging to this run | We remove it after the apps stop |
| **Host** | The host's shared memory, with normal access permissions | We leave it alone |

## Native settings and conflicts

Native mounts specify the host source, app-visible target and access permissions.
We apply those declarations to the launched container or unit. The source Compose
and unit files remain unchanged.

An app can mount one host segment read-only while keeping the rest of its shared
memory private to the run. Host updates remain visible through that live mount.
Explicit private shm settings remain app choices and may separate sharing groups.

Access to the entire host shm, including Docker `ipc: host`, requires global
**Host** mode. In **Isolated** mode, preparation rejects these settings with an
error naming the app, setting and required mode. Other incompatible IPC or mount
settings also fail before launch; preparation never silently skips run isolation.

Selective access belongs in native mounts. The global choice is one boolean field
on the existing `RunPlan`; it needs no settings message or `host_shms` list.

## Host shm in experiment snapshots

The existing A/B snapshots capture host shm metadata as well as the mount
declarations. Capture records object identity, size, access metadata and timestamps
without reading or copying shm contents. Work scales with the number of objects
observed, independently of their byte sizes.

These observations describe live inputs at capture time; they cannot establish
content equality. Normal host writes keep Review valid. Mount or configuration
changes still require fresh preparation.

## Cleanup follows ownership

The kernel manages shared-memory objects. An app exiting does not authorize
tearing down the run's shared mounts or namespaces while other apps use them.
After verified shutdown, the backend releases the resources it created for the
run. Host memory and externally owned mounts remain. If shutdown is unconfirmed,
we retain run resources and report incomplete cleanup.

See the [implementation design and work plan](110826-run-shared-memory-impl.md)
for how to build and test this. This design replaces [#206](https://github.com/Cutuy/doramcap/pull/206).
