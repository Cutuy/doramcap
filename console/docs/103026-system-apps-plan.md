![Console, workflow and step boundaries](103026-console-compose-steps.svg)

# Reusable system apps: parallel implementation specification

Created: 2026-10-30. Audience: implementation agents and the integration owner.

Extend existing app backends so Console can inject useful supporting services into
one Compose experiment. Deliver independently reviewable changes while the Dora
and Console stacks continue landing. This specification defines behavior and
ownership; adapt implementation details to the current upstream contracts.

## Product scope

| Component | Selection | Result |
| --- | --- | --- |
| Dora coordinator and daemon | Required, selected and locked | Shared communication services and IPC |
| Dora MCAP recorder | Optional | Selected workload outputs recorded to MCAP |
| ROS 2 MCAP recorder | Optional | Selected ROS 2 topics recorded to MCAP |
| Live bridge | Optional, selected by default | Declared app outputs relayed to Console ([plan](110126-live-bridge.md)) |
| CPU monitoring and recording | Optional | Live Console metrics and dedicated CPU MCAP |
| Fault Watchdog | Optional | Fault aggregation, with configurable MCAP/OTLP outputs |

Expose no demo apps. CPU Viewer's backend supports CPU monitoring; its gallery UI
is not a separate Console selection. Reuse and evolve existing backends, protocols,
recording libraries and deployment inputs before creating another implementation.
CPU and workload recording can run together with separate outputs.

## Moving upstream dependencies

At creation, the open chains are:

- Dora: #30 → #31 → #25 → #32 → #28 → #29 → #15.
- Console: #20 → #21 → #22 → #14 → #16.

These are navigation hints. Before implementation and again before publication,
fetch current refs, inspect PR state/base/head and identify the actual API owner.
Record the base SHA and relevant dependency SHAs in each handoff. Do not assume
branch tips, PR boundaries or file locations stay fixed.

The expected architecture is one assembled experiment project, service startup
stages, shared Dora setup, generated Compose types and explicit recordings.
The old `SystemAppsConfig`, CPU recipe injection, multiple runtime project groups
and `SnapshotGroup` must not become dependencies of new work.

Start backend work from current main where it compiles independently. Read the
Dora tip for compatibility. If a needed contract exists only in an open PR,
identify that exact dependency and base the change on its owner; do not copy its
implementation into another PR. Rebase dependent changes after squash merges.
Use generated protobuf types directly where available.

## Coordination and ownership

Assign one owner to each workstream below. Run independent streams concurrently
within the available worktree and agent capacity; the integration owner can be
one of those agents after its backend work is published.

- Follow repository AGENTS.md and the worktree-pool skill. Preserve dirty main
  checkout files and other agents' leases. Keep implementation in leased checkouts.
- Each agent edits its owned files. Shared Cargo wiring, schemas and deployment
  files require an assigned owner before concurrent edits. Propose cross-owner
  changes in the handoff rather than rewriting another stream's work.
- Coordinate existing gallery Compose, graph and registration edits with #29/#15.
  Prefer reusable deployment fragments alongside those files while they are active;
  let their owner handle gallery migration. Keep ordinary backend code Console-free.
- Prefer Rust/TypeScript and protobuf/pbtxt configuration. Present new third-party
  dependencies for evaluation before adopting them. Do not add compatibility
  aliases, proto reservations, generic plugin frameworks or replacement daemons.
- Give each reusable backend change its own small PR. Keep Console integration
  separate. Do not merge another owner's PR or message another task without the
  user's authorization.

Before dependent work, agree on the smallest concrete configuration and output
contract: input names, topic/schema encoding, paths, readiness, shutdown behavior
and loss reporting. Record it beside the owning API. Other agents consume that
contract; they must not invent parallel types or commit placeholder integrations.

## Agent A: reusable Dora MCAP recorder

Own `apps/mcap-recorder`, its new configuration schema, and changes needed in
`libs/telemetry/mcap`. Coordinate shared dependency wiring with the integration owner.

Existing code recognizes only CPU `snapshots` and `dictionary` inputs and embeds
the CPU telemetry schema. The gallery deployment also launches a CPU producer.

- Extend the existing recorder with pbtxt configuration for input-to-topic mapping,
  payload encoding, schema information and output destination.
- Support configured channels and preserve CPU snapshot/dictionary validation.
  Define how supported Dora payloads are serialized; reject unsupported types
  explicitly. Preserve available source timing and metadata, distinguishing receipt
  time when the producer supplies no timestamp. Do not invent source sequence data.
- Reuse the existing writer implementation and established storage APIs. Keep one
  recorder implementation usable for CPU and user workloads, with separate instances.
- Provide a recorder-only Compose input and graph fragment, with configurable node
  identity, queues and paths. Do not launch a producer or modify user source code.
- Preserve create-new output behavior, graceful input drain, finalized recordings
  and readable partial evidence. Expose failures and known drops; do not claim
  lossless recording merely because the file has a valid footer.

Acceptance: record at least two differently configured topics with their schema
and payload intact; retain CPU recording behavior; reject invalid configuration
and existing output paths; drain normal closure and report interrupted recording.
Validate the configured queue policy with slow storage. Hand off configuration
examples, supported encodings, artifact contract and graph wiring requirements.

## Agent B: reusable CPU collection

Own `apps/system-monitor`, `apps/cpu-viewer/server` and reusable CPU deployment
inputs. Agent A owns the recorder and MCAP schema/writer changes.

Existing collection, telemetry schemas and CPU Viewer's bounded subscriber/feed
server are reusable. The gallery deployment bundles monitor, recorder and viewer.

- Reuse one system-monitor producer for live display and dedicated CPU recording.
  Configure backend, capacity, node identity and output locations explicitly.
- Declare the monitor's `snapshots` and `dictionary` outputs as `live_outputs`;
  the [live bridge](110126-live-bridge.md) delivers them to Console.
- Provide composable monitor inputs using Agent A's recorder contract. Keep
  BPF privileges on the collector; the recorder must not inherit them.

Acceptance: one producer feeds both live delivery and a finalized CPU recording;
a slow/disconnected frontend does not halt collection; required dictionaries and
loss information remain available. Existing standalone gallery behavior remains
usable. Verify capability-dependent collection separately from ordinary unit tests.

## Agent C: standalone Fault Watchdog

Own `apps/fault-watchdog` and its reusable deployment inputs. Coordinate any changes
in `libs/fault/*` with their existing owners.

The backend already has configurable Dora inputs, protobuf/pbtxt configuration,
bounded publication, MCAP rotation and OTLP sinks. Its deployment bundles synthetic
reporters, controls and a dashboard.

- Add a watchdog-only Compose input and graph fragment using the existing binary.
- Configure input names, liveness policy and output destinations through the
  existing schema. Enable existing MCAP/OTLP implementations as selected.
- Accept workload producers that implement the existing fault protocol. Document
  that connection requirement; do not interpret arbitrary payloads as fault reports.
- Define health, fault output and recording artifacts for Console consumption.
  Reuse existing bounded publication and sink drain behavior.

Acceptance: run against an external protocol-compatible producer without demo
services; observe faults and source timeout/disconnect; validate MCAP and OTLP
options, sink failure reporting and graceful shutdown. Fixtures may simulate a
producer, but must not become injected production services.

## Agent D: ROS 2 recording integration

Own a small ROS 2 recorder deployment/configuration package under `apps/`, with its
name agreed before creating it. No Console, Dora runtime or CPU writer edits.

No ROS recorder integration currently exists here. Evaluate upstream rosbag2 with
its MCAP plugin; reuse it rather than building another ROS subscriber/writer.

- Propose a supported ROS distribution/image and message-type provisioning method.
  Present dependency/image choices before adopting them. Continue specification
  and configuration work while a required choice is pending.
- Configure topic selection, discovery/domain settings, middleware compatibility,
  QoS overrides and output location. Keep ROS deployment configuration separate
  from Dora graph configuration; ROS topics need not be bridged through Dora.
- Define observable readiness, matched topics, recording loss and graceful
  finalization. Make unavailable data explicit rather than manufacturing health.
- Package the recorder as ordinary Compose input. Document external discovery,
  custom type and intra-process/SHM constraints without requiring user source
  changes for ordinary supported topic recording.

Acceptance: record a supported external ROS 2 producer into readable MCAP with
schema/type information; test topic filtering, discovery/QoS mismatch and graceful
shutdown. Record the supported distribution/RMW/type combinations actually tested.
Do not claim arbitrary ROS deployments are supported from one fixture.

## Integration owner: Console and the assembled experiment

Own `console/` and coordinate shared workflow changes with the Dora chain owner.
Contract review can start immediately; defer runtime wiring until its actual
upstream dependency is available and the backend contracts are settled.

- Replace Console's old CPU-specific configuration with a host-approved pbtxt
  catalog. Reuse generated Compose/Dora types; browser commands select approved IDs.
- Render required Dora services as selected and locked. Validate optional IDs,
  defaults and required configuration on the backend. Exclude gallery demos.
- Translate selection into explicit first-party inputs, graph connections and
  recording declarations for the existing single-project workflow. Reject service,
  node and output collisions, missing connections and incompatible configurations.
- Adapt configuration comparison to the assembled project, startup service stages,
  selected graphs/settings and outputs. Remove `SnapshotGroup`; preserve comparison
  across generated artifact directories without hiding meaningful changes.
- Relay declared live outputs through the live bridge and expose per-app state and per-recording
  status/downloads. Remove fixed CPU file locations and the single-recording limit.
- Reuse workflow-owned readiness/teardown notifications and final cleanup evidence.
  Notification listener loss never stops execution. Explicit workflow Stop exits
  observation and proceeds through cleanup. Add no step-cancellation or executor
  shutdown feature; document any inherited startup interruption assumptions.
- Keep app identities and UI lifecycle concepts out of generic orchestration.
  Preserve the existing ownership and recovery rules from the landed Dora chain.

Acceptance: required services cannot be deselected; no optional apps is valid;
each optional app works alone when its inputs are available; CPU and workload
recording coexist; invalid combinations reject before launch. Verify stale-input
rejection, automatic-failure Stopping status, explicit stop/drain, separate output
publication and incomplete-recording/recovery behavior.

## Validation and handoff

Use focused checks while these stacks move; do not start full CI or ARM64 runs
for this work without a new request. Follow repository release/ci-profile and
locked-dependency rules. Use Arrange/Act/Assert in new behavioral tests. Validate
application behavior and failure cases, rather than protobuf's own encoding.

Each agent's handoff must include:

- PR URL, exact source/base SHAs, owning files and actual upstream dependencies.
- Final public configuration/API and a small working deployment example.
- Passing checks and runtime evidence, with ignored, unavailable or deferred cases
  listed explicitly. Capability/Docker/ROS availability is not implied by compilation.
- Remaining integration work, incompatible upstream changes and any pending choices.

Before landing, re-check both chains and update callers against their current
contracts. If upstream changes alter the design, describe the concrete difference
and adjust the owned implementation. Do not preserve obsolete multi-project or
special CPU paths merely to avoid rebasing. The integration owner verifies the
combined selection/recording behavior after the prerequisites land.
