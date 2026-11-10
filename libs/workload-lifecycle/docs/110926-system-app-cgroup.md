![System-app cgroup](110926-system-app-cgroup.svg)

# System-app cgroup

Created: 2026-11-09. [Implementation](110926-system-app-cgroup-impl.md).

Each experiment has a system-app accounting boundary, including Dora and the
shared-memory holder. Nested cgroups contribute to its CPU and memory accounting.
Existing system/user app inputs select placement.

`workload-lifecycle::cgroup::SystemAppsCgroups` owns kernel identity, accounting
availability, membership checks, artifact persistence and cleanup verification.
Its contract contains a host cgroup path and kernel ID. Backend mechanics establish
the group and report whether cleanup removed it or retained an empty runtime parent.

Compose supplies `cgroup_parent` using Docker's configured cgroupfs or systemd driver.
Docker creates that parent before container execution. After system apps start, the
shared step records the group and verifies host PIDs before user startup. Each run
uses a distinct parent tied to its fresh artifact directory. Compose uses Docker APIs and requires cgroup v2 CPU/memory
accounting; its workflow has no host D-Bus dependency.

The native systemd backend creates slices through its existing service manager.
Supervisor uses its existing filesystem allocation and durable ownership ledger;
its delegated parent must enable memory accounting for children.
Both reuse the shared step and membership checks.

After writers stop and evidence is finalized, directly owned groups are removed.
Empty parents owned by Docker's runtime may remain; cleanup verifies their emptiness.
Startup failures use existing shutdown/recovery paths. Identity artifacts remain
with run evidence as `system-app-cgroup.pb`, encoded directly from the generated
`SystemAppCgroup` message. Generated messages stay in Cargo's build directory;
binary run artifacts stay in each experiment's artifact directory.
Monitor collectors remain separate work.
