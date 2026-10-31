# Robotics adoption feasibility

Updated 2026-10-29. The Docker baseline and five robotics entries were inspected
in source; remaining
entries preserve the user-provided SPA survey as qualification targets. No project
was executed. Source compatibility does not establish runtime support.

## Pilot sequence

| Pilot and pinned source | Observed behavior | Design consequence / open gate |
|---|---|---|
| **First:** [Docker nginx-golang](https://github.com/docker/awesome-compose/tree/master/nginx-golang) (revision to pin) | NGINX and Go HTTP backend; dependency ordering, network, build and read-only config bind | Qualify release-only adoption on a clean GPU-less VM: no Doramcap checkout or build tools, HTTP response, CPU MCAP, cleanup and repeat. See [acceptance plan](../../../apps/project-explorer/server/docs/102726-experiment-preparation.md#m4--release-only-adoption-on-a-clean-vm). |
| **Later robotics pilot:** [OpenPI LIBERO Compose](https://github.com/Physical-Intelligence/openpi/blob/215abfb217dbac7d5f1273282331b9b1866c0479/examples/libero/compose.yml) | Separate GPU-backed policy server and LIBERO evaluator; privileged simulator, host networking, source/data/cache binds and X11 socket | Qualify the upstream baseline, model readiness and evaluator completion; preserve evidence and verify both services stop. See [qualification plan](../../../apps/project-explorer/server/docs/102726-experiment-preparation.md#m5--later-robotics-qualification-openpi-libero). |
| [vEDGAR Compose](https://github.com/TUMFTM/vEDGAR/blob/5f6ea693c73ffd43d4b52ffde216fb6a56ee3bbc/docker-compose.yml) | Profiles, selected GPUs, host networking, privileged CARLA, source/output bind mounts, interactive shell service | Begin with `singlepc`; lease ports/GPU/output paths, capture mounted code, define CARLA readiness/reset. HiL requires a later multi-host coordinator. |
| [AnywhereVLA launcher](https://github.com/SelfAI-research/AnywhereVLA/blob/674cd7840b9439f9420bf175bf66110b62b61210/scripts/run.sh) and [CPU Compose](https://github.com/SelfAI-research/AnywhereVLA/blob/674cd7840b9439f9420bf175bf66110b62b61210/src/cpu_part/docker-compose.yaml) | Multiple project directories, platform-dependent files, `extends`, foreground/background host Python bridges | Preserve project boundaries and base paths. Host helpers require an additional executor or explicit packaging. Validate filenames and startup intent before execution. |
| [Event-Orin simulation](https://github.com/tudelft/event_orin_drone/blob/1a25c6835476d4ea6578cf7b0aab6ea854260b23/sim/px4_gazebo_sim_external_modes.sh) and [recorder](https://github.com/tudelft/event_orin_drone/blob/1a25c6835476d4ea6578cf7b0aab6ea854260b23/ros/src/eo_drone/launch/drone_flight_recorder.launch.py) | tmux launches host programs and a container after a fixed delay; ROS camera and MCAP recorder share a component process | Translate stages with explicit readiness. Preserve composition; qualify drain/close inside the shared process. A generic Compose importer cannot infer these contracts. |
| [OpenTeach worker factory](https://github.com/aadhithya14/Open-Teach/blob/32a7d44b33953066ff27312a7b2b4c294f4f52c5/openteach/components/initializers.py), [teleop](https://github.com/aadhithya14/Open-Teach/blob/32a7d44b33953066ff27312a7b2b4c294f4f52c5/teleop.py), [collection](https://github.com/aadhithya14/Open-Teach/blob/32a7d44b33953066ff27312a7b2b4c294f4f52c5/data_collect.py) | Hydra-selected multiprocessing workers; separate teleop/collector roots; `start`/`join`; episode output directory reuse | Own each root's containment boundary, retain child observations, and add readiness/finalization evidence. Direct host adoption needs a subprocess/systemd adapter or deliberate container packaging. |

### Concrete import gaps

- OpenPI's `depends_on` orders starts; upstream owns application readiness.
  The evaluator completes while the server remains alive; use the runner's stop
  future. Preserve declared X11, device and GPU access through adoption.
- AnywhereVLA's inspected launcher checks names containing `docker compose`, while
  the corresponding repository files use `docker-compose`. Foreground Compose
  invocations can block later stages; detached mode skips host bridges. Preserve
  this finding as an import diagnostic and define a reviewed adoption recipe.
- vEDGAR's inspected Compose file lacks application readiness dependencies.
  Profile resolution alone cannot supply simulator or sensor readiness.
- Event-Orin's recorder has a 100 MiB cache and starts recording immediately.
  The launch file alone supplies no proof of drain completion; inspect the
  recorder implementation and qualify its shutdown before claiming clean capture.
- OpenTeach creates multiple recorders and reuses an existing demonstration
  directory. Assign session-specific storage and inspect worker shutdown handlers
  before claiming all output is complete.

## Lifetime qualification for every candidate

Native monitors/writers and adopted foreign workloads must have fresh instances
for each session. Verify their exit before admitting the next experiment. Existing
application boundary hooks support readiness/drain/reset within that lifetime.

For upstream persistent servers, first evaluate launching a dedicated instance per
session and disabling competing boot/restart ownership. If infeasible, document an
explicit daemon exception, retained state, reset checks and isolation limitations.
This is a compatibility gap to resolve, rather than an assumed supported lifetime.

## Remaining survey targets

These are design hypotheses from the supplied research, pending pinned launch-file
review. Their inclusion does not assert Docker packaging or support.

| Project | Required adoption boundary | Evidence needed next |
|---|---|---|
| [Walk These Ways](https://github.com/Improbable-AI/walk-these-ways) | Container replacement and controller activation | Exact replacement ownership; calibration/readiness and safe stop |
| [OmniNxt](https://github.com/HKUST-Aerial-Robotics/OmniNxt) | Container plus ROS launch; first-run engine build | Build-cache identity, warmup completion and launch hierarchy |
| [Mobile ALOHA](https://github.com/MarkFzp/mobile-aloha) | Shared robot services plus episode scripts | Arm/camera ownership, recording drain and arm shutdown hooks |
| [BeyondMimic](https://github.com/HybridRobotics/motion_tracking_controller) | ROS respawn and controller states | Inner restart owner; activation/deactivation acknowledgment |
| [AutoSDV](https://github.com/NEWSLabNTU/AutoSDV) | Versioned launch/configuration and diagnostics | Launcher interface; minimum-risk behavior during teardown |
| [Hydra-ROS](https://github.com/MIT-SPARK/Hydra-ROS) | Mapping/sensor group and final graph artifact | Save trigger, completion acknowledgment and artifact integrity |
| [KR Autonomous Flight](https://github.com/KumarRobotics/kr_autonomous_flight) | Flight state machine and separate recorder | Readiness and shutdown dependency graph; flight-safe state |
| [AutoEval](https://github.com/zhouzypaul/auto_eval) | Queue dispatch attempt plus persistent servers | Session-scoped server feasibility; dispatch identity, worker reconciliation and cooldown/reset |
| [DROID](https://github.com/droid-dataset/droid) | Persistent robot service plus collector | Session-scoped server feasibility; boot ownership, explicit daemon exception if needed, recorder drain |
| [HIL-SERL](https://github.com/rail-berkeley/hil-serl) | Nested controller replacement and per-arm servers | Replacement identity, reset endpoint semantics and arm leases |
| [UMI](https://github.com/real-stanford/universal_manipulation_interface) | Multiprocessing readiness/start/stop | Hook acknowledgments, child containment and recording completion |
| [JSK robot stack](https://github.com/jsk-ros-pkg/jsk_robot) | Supervisor ordering and ROS-master readiness | Restart delegation and coordinated supervisor shutdown |

## Zero-code adoption gate

For every Compose candidate, add a service to the supported project and rerun the
same workflow without any orchestration, pipeline or step-registry code change.
Verify discovery under existing profile/target selection, automatic evidence
membership, policy application and cleanup. Missing semantic hooks are reported
as coverage limits; customers must not maintain a duplicate workload list.

## Qualification tiers

1. **Launch adoption:** source resolution, owned runtime objects and bounded stop.
2. **Experiment boundary:** preparation, readiness, recording drain, verified reset and exit of every session-owned native/foreign process.
3. **Semantic evidence:** child health, controller states and message/output provenance.

Report the tier per project and backend. The proposed pilot order exercises these
boundaries progressively; it does not imply that each project is Compose-native.
