I inspected the backends and incoming Dora deployments. The useful Console options are **Dora infrastructure, Dora recording, ROS 2 recording, CPU monitoring/recording, and Fault Watchdog**. Demo applications stay out.

**Dora coordinator + daemon — required**

The incoming workflow already owns their startup, shared IPC and shutdown. [Existing implementation](https://github.com/Cutuy/doramcap/blob/codex/shared-dora-implementation/libs/workload-docker/src/workflows/shared_dora.rs)

- Replace Console’s old `SystemAppsConfig` integration with `DoraRuntime`.
- Show both services as required, selected entries with readiness/failure status.
- Include resolved communication settings and graph contents in configuration comparison.

**Dora MCAP recorder — optional**

Extend the existing `mcap-recorder`. Both its input handling and `telemetry_mcap::McapSink` currently assume CPU snapshot/dictionary messages. [Recorder backend](https://github.com/Cutuy/doramcap/blob/codex/shared-dora-implementation/apps/mcap-recorder/src/main.rs)

- Add protobuf/pbtxt configuration for input-to-topic mapping, encoding/schema information and output path.
- Extend the writer to support configured channels and payload types while preserving CPU recording.
- Provide a recorder-only Compose deployment; the gallery deployment currently includes a CPU producer.
- Add selected outputs to the generated Dora graph; configure queue limits and drain recorders before finalization.

**ROS 2 MCAP recorder — optional**

There is no existing ROS recorder integration in this repository. Reuse **rosbag2 with its MCAP plugin**, which already provides topic recording and storage. [Upstream implementation](https://github.com/ros2/rosbag2/blob/rolling/rosbag2_storage_mcap/README.md)

- Package an approved ROS distribution with rosbag2, MCAP support and required message-type packages.
- Configure topics, discovery/domain settings and QoS overrides.
- Add recorder readiness, graceful shutdown and finalized artifact collection.
- Report incompatible types, unmatched topics and recording loss through Console. [Rosbag2 configuration](https://github.com/ros2/rosbag2/blob/rolling/README.md)

**CPU monitoring + dedicated recording — optional**

Reuse `system-monitor`, the existing CPU recorder, and CPU Viewer’s subscriber backend. The viewer backend already validates telemetry and serves a bounded live feed; we should evolve it for reuse. [CPU feed backend](https://github.com/Cutuy/doramcap/blob/codex/shared-dora-implementation/apps/cpu-viewer/server/src/main.rs)

- Provide composable deployments for one monitor, its recorder and its live-feed service.
- Configure separate node identities and `cpu.mcap`, avoiding collisions with workload recording.
- Expose the existing live-feed backend without requiring the gallery frontend; connect Console’s CPU display to it.
- Replace Console’s fixed `cpu.pb`/`cpu.mcap` assumptions with declared feed and artifact locations.

CPU Viewer need not become another checkbox: its backend supports the CPU monitoring option.

**Fault Watchdog — optional**

The backend is already reusable: protobuf configuration, configurable Dora inputs, bounded publication, and optional MCAP/OTLP sinks exist. The deployment is what remains bundled with synthetic sources, controls and a dashboard. [Watchdog backend](https://github.com/Cutuy/doramcap/blob/codex/shared-dora-implementation/apps/fault-watchdog/src/main.rs)

- Add a watchdog-only Compose deployment and configurable graph connections.
- Supply workload-specific input names and liveness/output settings through existing pbtxt configuration.
- Expose its existing MCAP and OTLP options; collect recordings and surface health/output failures.
- Validate that selected producers emit the supported fault protocol. Arbitrary workload messages cannot automatically become fault reports.

**Shared Console work**

- Define approved app entries and defaults in protobuf/pbtxt; browser requests select IDs.
- Assemble selected components into one experiment project and validate service/node/output conflicts.
- Adapt configuration snapshots to that single project; remove `SnapshotGroup`.
- Display per-app status and per-recording artifacts using the existing workflow’s cleanup evidence.

These are proposed work items; no code changes made.
