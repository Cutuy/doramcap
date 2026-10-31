# Component ownership

| Location | Owns |
|---|---|
| `libs/<crate>/` | Standalone reusable Rust crates |
| `libs/<domain>/<crate>/` | Related crates grouped under resources, telemetry, memory or fault |
| `apps/<app>/` | System apps, demos and their supporting crates/frontends |
| `console/docs/` | Planned Console architecture and milestones |
| `docs/` | Repository-wide vision, status and conventions |
| `scripts/` | Workspace development and CI tooling |

The [root README](../README.md) and [library index](../libs/README.md) provide
component entry points. Domain directories keep a short index README; leaf
crates own `Cargo.toml` and API documentation. Cargo workspace membership is
explicit, including nested crates: add new paths to the root `Cargo.toml`.

## Keep details with their owner

- Each component owns its docs, tests, examples and benchmarks.
- Dated plans use `component/docs/mmddyy-feature-name.md`, with architecture SVGs
  alongside them and work/validation checklists per milestone.
- Library APIs are documented beside the code. Root docs link to component evidence.
- Console implementation is planned under `console/{frontend,server}`; current
  explorer and demo apps retain their own paths.

## Current integration boundaries

| Owner | Responsibility |
|---|---|
| `libs/orchestrator` | Observable async steps and workflow telemetry |
| `libs/workload-lifecycle` | Shared recorded-run sequencing and recording checks |
| `libs/workload-docker` | Compose, Dora wiring, filesystem isolation and cleanup |
| `libs/workload-supervisor` | Owned Supervisor/System Monitor runs |
| `libs/dora-app` | Shared Dora attachment, environment bridge and readiness |
| `apps/system-monitor/dora` | CPU publication and direct-buffer Protobuf encoding |
| `apps/cpu-viewer/server` | HTTP/WebSocket delivery and viewer schema |
| `apps/cpu-viewer/dataflows` | Monitor/viewer/recorder demo compositions |
| `apps/mcap-recorder/tests` | Independent recorder lifecycle tests |
| `apps/system-monitor/tests` | Native collector integration tests |
| `libs/telemetry/protocol` | CPU schemas, codecs and descriptors |
| `libs/telemetry/workflow-protocol` | Workflow event contracts |

Browser-generated bindings stay in the consuming frontend. Apps own runtime
interfaces and keep declarative gallery wrappers under `apps/<app>/deploy/`.
Use the [Docker-hostable app guide](../apps/project-explorer/server/docs/docker-hostable-apps.md)
when adding an app.

Commands run from the repository root unless stated otherwise. Dora YAML
executable paths are relative to the YAML file. Follow [AGENTS.md](../AGENTS.md)
for leases, configuration rules, build profiles and validation.
