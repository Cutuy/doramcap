![Service names reach the browser in Status](110526-service-names-in-cpu-consumers.svg)

# Service names in Top CPU consumers

Created: 2026-11-05. Status: proposed; the name source awaits the owner's decision.

Top CPU consumers shows `docker-<id>.scope` in the installed Console and service names
in the demo. Show the service a container belongs to, in both.

## Current behavior

- The monitor publishes cgroup IDs and paths; it has no knowledge of Compose.
- The live bridge relays those frames unmodified, and the browser shows the path's last segment.
- The Compose adapter's `inspect()` already returns each container's ID and service.
- `ExecutionUpdate` already carries service facts to the server, such as `stopping_services`.

## Name source: awaiting decision

| Option | Change | Tradeoff |
| --- | --- | --- |
| **Status (proposed)** | Adapter facts flow through `ExecutionUpdate` and `Status`; the browser labels matching groups. | One data path; frames stay unmodified. Recordings keep raw paths. |
| Server asks Docker | The TypeScript server lists containers by label. | No Rust change, but a second source of truth with no event to say when to ask. |
| Monitor resolves names | The monitor reads Docker and publishes service names. | Recordings gain names, but a generic BPF monitor gains a Docker dependency. |

## Behavior

- A group shows its service name when its path contains a known container ID.
- Other groups keep their path segment, so host processes stay visible and honest.
- A restarted service keeps its name on each container, listed as separate rows.
- The list includes system app containers, by their service names.
- Applies to the live panel. Recording contents are unchanged.

## Milestone 1: carry container identity

**Work items**

- [ ] Trace how inspection results reach the native update; publish on start and on change, without timed polling.
- [ ] Add a repeated `ServiceContainer { container_id, service }` message to `ExecutionUpdate` and `Status`.
- [ ] Retain every container seen during a run, including restarted ones.
- [ ] Cover both Compose projects if [system and user isolation](https://github.com/Cutuy/doramcap/pull/175) lands first.

**Validation**

- [ ] Native and server tests with a real Compose project, using capability rules from `AGENTS.md`.
- [ ] Status shows each service's container after start and after a restart.

## Milestone 2: label the panel

**Work items**

- [ ] Match container IDs in cgroup paths for the systemd and cgroupfs drivers.
- [ ] Label Top CPU consumers from Status, keeping groups distinct by ID.
- [ ] Make the demo supply the same Status field and real-format `docker-<id>.scope` paths, replacing its made-up paths.

**Validation**

- [ ] Frontend tests for both drivers, unmatched groups and a restarted service.
- [ ] A real run on x86_64 and ARM64 shows service names; a host process keeps its path.
- [ ] Demo walk in light and dark.
