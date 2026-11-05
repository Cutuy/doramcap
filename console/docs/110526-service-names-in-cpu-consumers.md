![The system monitor adds service names to its dictionary](110526-service-names-in-cpu-consumers.svg)

# Service names in Top CPU consumers

Created: 2026-11-05. Status: name source decided; Docker access awaits the owner's decision.

Top CPU consumers shows `docker-<id>.scope` in the installed Console and service names
in the demo. Show each container's Compose service in both, and in recordings.

## Accepted design

Agreed with the project owner on 2026-11-05.

| Area | Decision |
| --- | --- |
| Name source | The system monitor resolves names and publishes them in its cgroup dictionary. |
| Reach | Live panels and MCAP recordings gain names through the existing dictionary topic. |

## Docker access: awaiting decision

The Console's `cpu-monitor` runs read-only with `cap_drop: [ALL]` plus BPF, PERFMON and SYSLOG, and has no Docker access.

| Option | Change | Tradeoff |
| --- | --- | --- |
| **Docker socket (proposed)** | Mount the socket into `cpu-monitor`; query only container labels. | Simplest and exact. A read-only mount does not limit the API, so a compromised monitor has root-equivalent control of the host. |
| Filtering socket proxy | A proxy container allows only container inspection. | Limits the API, at the cost of another image and service on every run. |
| Docker metadata files | Mount `/var/lib/docker/containers` read-only and read `config.v2.json`. | No API access, but relies on a private format, needs a read capability, and exposes every container's environment secrets. |

## Behavior

- A group's name is its `com.docker.compose.service` label, in a new `DictionaryEntry` field; `path` stays unchanged.
- Groups without that label keep their path segment, so host processes stay visible.
- Names follow the dictionary's existing 2 s checkpoints, so a new service can briefly show its path.
- A failed or missing Docker connection leaves paths in place, is counted in health output, and never stops collection.
- Standalone monitors stay Docker-free: resolution is off unless the configuration enables it.
- Restarted services appear as separate rows with the same name.

## Milestone 1: resolve names in the monitor

**Work items**

- [ ] Add a `service` string to `DictionaryEntry`; existing readers ignore it.
- [ ] Add a `container_names` section to `monitor.proto`, off by default.
- [ ] Run lookups on a dedicated thread with a bounded channel; the dictionary sends new container IDs and picks up replies at its next checkpoint.
- [ ] Parse container IDs from systemd and cgroupfs paths, retrying failures at later checkpoints.
- [ ] Use the workspace's pinned `bollard`, pending owner review of adding it and an async runtime to the monitor.

**Validation**

- [ ] Unit tests: both path forms, unlabeled containers, a failed lookup and a full channel.
- [ ] Native test against real Docker: service names appear; collection stays at 10 Hz while Docker is stopped.
- [ ] A recording's dictionary topic contains names, and an older reader still decodes it.

## Milestone 2: ship it in Console

**Work items**

- [ ] Enable resolution and the chosen Docker access in the `cpu-monitor` system app.
- [ ] Document the access in the system app specification.
- [ ] Prefer `service` over the path segment in Top CPU consumers, keeping groups distinct by ID.
- [ ] Make the demo publish real-format paths plus `service`, replacing its made-up paths.

**Validation**

- [ ] Frontend tests: named, unnamed and restarted groups.
- [ ] A real Console run on x86_64 and ARM64 shows service names, live and in the MCAP; a host process keeps its path.
- [ ] Demo walk in light and dark.
