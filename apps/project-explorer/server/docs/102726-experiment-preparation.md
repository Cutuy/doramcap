![Experiment preparation](102726-experiment-preparation.svg)

# Prepare and run an existing Compose experiment

Created: 2026-10-27. Planned frontend/server work.

Users select existing Compose projects, resolve setup questions once, then run and
repeat an existing workload with lifecycle and recording evidence. Upstream files
and application code stay unchanged. Preparation produces the typed `ComposeRun`
input for the procedural Rust workflow in `workload-docker`.

The gallery server owns interactive preparation and progress delivery. The core
orchestrator owns step execution and telemetry. Docker resolution, mount planning,
launch and cleanup remain in `workload-docker`. The browser never owns cleanup.

## M1 — Select and resolve inputs

### Work items

- [ ] Add “Run an experiment” with one or more existing Compose projects.
- [ ] Select ordered files, working directories, env inputs, profiles and services.
- [ ] Resolve through Docker Compose; retain source identities and resolved settings.
- [ ] Surface missing images, devices, permissions and ambiguous completion/readiness.
- [ ] Save reusable preparation inputs; resolve again when their sources change.

### Validation

- [ ] Adding a service to upstream Compose requires zero pipeline or application edits.
- [ ] Multiple files in one project remain distinct from multiple independently named projects.
- [ ] Profile/env changes are reflected in the resolved preview; secrets are redacted.
- [ ] Errors identify the project, source and actionable prerequisite.

## M2 — Explain the effective launch

### Work items

- [ ] Show original → effective settings and the reason for each override, including
  restart policies, image IDs, project names, labels and host bind sources.
- [ ] Show writable directory overlays, shared aliases, read-only inputs and fresh outputs.
- [ ] Choose fresh artifact storage outside every source tree; flag missing source
  directories before submission and explain the current existing-directory requirement.
- [ ] Offer write-back by default or independent retained results; explain when host
  changes become visible and what the next run inherits.
- [ ] Show named-volume carryover, container-layer disposal and unsupported mounts.
- [ ] Check capabilities, local daemon mount visibility and backing filesystem support.
  Actual mount preparation remains a workflow step and fails closed on error.

### Validation

- [ ] Source Compose/env files remain byte-for-byte unchanged across preparation and runs.
- [ ] Every runtime override is linked to persisted resolution or mount-mapping evidence.
- [ ] Qualified kernel-interface and device mounts retain their declared access;
  unknown sockets, writable single-file binds and nested mounts produce clear errors.
  Report all passthrough explicitly; never silently bypass selected isolation.
- [ ] No default hashing, model copying, artificial file-count cutoff or repeated prompt.

## M3 — Launch, observe and repeat

### Work items

- [ ] Submit prepared input to a server-owned serial runner; expose lifecycle telemetry.
- [ ] Show preparation, first-party startup, foreign startup, observation, shutdown,
  categorized diffs, filesystem finalization and cleanup.
- [ ] Surface existing Docker container health in a visible workflow observation
  step, including image-defined checks. Preserve Compose's own readiness semantics;
  services without a health check remain eligible and report process state only.
- [ ] Support explicit stop and browser reconnect; retain server-owned run identity.
- [ ] Show results, recording status, original/effective paths and incomplete cleanup.
- [ ] Reset old writers/resources before preparation; cleanup failures block launch.
- [ ] Reuse the hosting components in [on-demand apps](102727-on-demand-apps.md).
  Define experiment disconnect behavior explicitly: gallery idle-app expiry must
  never kill the lifecycle manager while it still owns an active run.

### Validation

- [ ] Two serial runs get fresh monitors, recordings and overlay upper/work directories.
- [ ] Disconnect/reconnect preserves the run and exposes current state and results.
- [ ] A failed start still drains owned workloads and records cleanup outcomes.
- [ ] Existing healthy, starting and unhealthy container states appear accurately;
  a service without a health check runs without a new application requirement.
- [ ] Busy mounts and incomplete evidence keep the next-run action unavailable.
- [ ] File lists load incrementally; preview size never limits collection completeness.
- [ ] Verify production build, schema validation and desktop/mobile browser behavior.

## M4 — Release-only adoption on a clean VM

Acceptance target: a user downloads a Doramcap release onto a modest Linux VM,
points it at a working Compose project, and gets useful recordings and repeatable
runs. The VM needs no Doramcap source checkout, Rust/Node toolchain, or local build.
The release includes prebuilt binaries, required UI/runtime assets, and default
configuration files where needed.

First sample: Docker's [nginx-golang](https://github.com/docker/awesome-compose/tree/master/nginx-golang).
NGINX forwards HTTP requests to a Go backend that returns “Hello from Docker!”.
Its Compose file exercises two services, dependency ordering, networking, a build,
and a read-only configuration bind. The sample has no declared health checks or
writable data volumes; qualify those separately. No run is qualified yet.

Start with a GPU-less Linux x86_64 VM: 2 vCPUs, 4 GiB RAM and 10 GiB free disk.
These are provisional qualification targets; measure usage before publishing
minimum requirements. Document supported distribution, kernel, Docker/Compose,
BPF/cgroup features and required privileges. Ordinary upstream image builds are
part of preparing the working sample.

### Work items

- [ ] Produce a versioned downloadable release with binaries, required assets,
  default configuration and checksums. Resolve packaged resources relative to
  installation paths; remove developer-checkout and build-directory dependencies.
- [ ] Write a short installation/run guide using only release contents and documented
  host prerequisites. Include privilege setup and actionable prerequisite checks.
- [ ] Pin the sample commit and record image identities, including build bases.
  Verify its upstream build; record the selected revision and exact commands.
- [ ] Provision a fresh VM without Doramcap source, development tools, or preseeded
  image/build caches. Record its OS, kernel, resources and installed prerequisites.
- [ ] Check out the sample, build/start it with upstream Compose, and verify its HTTP
  response. Stop the baseline deployment before adoption to free port 80.
- [ ] Download and verify the release, unpack it, and select the existing sample
  through onboarding. Run without editing sample code, Compose or configuration.
- [ ] Generate HTTP traffic, inspect CPU recording results using the shipped tools,
  stop cooperatively, and repeat. Record commands, release identity, logs and artifacts.

### Validation

- [ ] A fresh VM completes the documented path using the release and sample checkout
  alone. No Doramcap compilation, source files, developer caches or undocumented
  manual fixes are required; all runtime assets resolve from the installed package.
- [ ] Both upstream and managed runs return the expected HTTP greeting through NGINX.
  Sample files remain byte-for-byte unchanged across adoption and both managed runs.
- [ ] Two managed runs create distinct workload/monitor instances and readable CPU
  MCAPs containing samples attributable to the sample workload during HTTP traffic.
  Results are accessible through the released interface.
- [ ] Stop finalizes recordings and removes owned containers, networks and temporary
  mounts; port 80 is reusable. Unrelated host resources remain intact and the second
  run succeeds without manual cleanup.
- [ ] Missing permissions or required kernel features produce actionable errors.
  Exercise interrupted startup and stop; report cleanup failures and prevent a
  conflicting replacement until cleanup is verified.
- [ ] Record startup time, peak RAM and disk consumption, and total installed size.
  Publish measured supported requirements and evidence before marking this complete.
- [ ] Keep Docker health-state, writable-volume/overlay and kernel-mount validation
  in their dedicated fixtures; this sample's success does not qualify those features.

## M5 — Later robotics qualification: OpenPI LIBERO

Later pilot, after M4 passes: [OpenPI `215abfb`](https://github.com/Physical-Intelligence/openpi/tree/215abfb217dbac7d5f1273282331b9b1866c0479),
using its [LIBERO Compose example](https://github.com/Physical-Intelligence/openpi/blob/215abfb217dbac7d5f1273282331b9b1866c0479/examples/libero/README.md).
Source review only; no upstream or managed run has been qualified. OpenPI is
used here for a real model and simulation workload, not as evidence of physical
robot or leaderboard qualification.

The upstream example launches a GPU-backed policy server and a LIBERO evaluator.
It uses host networking, a privileged evaluator, X11, and source/data/cache binds.
Preserve upstream startup and device settings. `depends_on` does not establish
model readiness; the application owns its readiness behavior. Evaluator completion
does not stop the server; the existing cooperative stop future ends the experiment.

### Work items

- [ ] Prepare a suitable Linux x86_64 host with Docker/Compose, NVIDIA driver and
  container runtime, adequate disk and RAM. Record kernel, BPF/cgroup capability,
  GPU/VRAM, driver, toolkit, storage and available space. Upstream documents
  Ubuntu 22.04 and more than 8 GB GPU memory for inference; qualify actual usage.
- [ ] Resolve the pinned repository and submodules; build images and obtain the
  selected checkpoint before launch. Record Compose version, image and base-image
  identities, checkpoint identity, task suite, seed, trial count and arguments.
- [ ] Run the upstream example independently first. Identify a bounded episode
  configuration, upstream startup behavior, evaluator exit/result and retained logs/videos.
  Measure startup, peak memory and disk; record observed failures and setup steps.
- [ ] Prepare the same working upstream deployment through onboarding. Preserve GPU
  requests, host networking, privileged mode and device/socket binds; show mount
  and cache disposition. Classify passthrough explicitly in the resolved plan.
- [ ] Declare `runtime` as an expected successful completion in Doramcap's prepared
  inputs; upstream Compose has no completion condition. Account for the current
  `completed_services` behavior, which waits for evaluator exit during startup:
  record a readiness budget covering the bounded evaluation, or separate startup
  readiness from expected completion before qualification.
- [ ] Run through the managed workflow without changing upstream files. Start fresh
  monitoring, verify evaluator-to-policy communication and completed episodes,
  then stop the server and finalize CPU MCAP and declared evaluation evidence.
- [ ] Document the supported host, task, exact launch inputs, prerequisites, known
  exceptions and recovery procedure. Keep vEDGAR, AnywhereVLA, Event-Orin and
  OpenTeach as later adoption targets in the [survey](../../../../libs/workload-lifecycle/docs/102626-adoption-candidates.md).

### Validation

- [ ] Baseline: verify real model inference and bounded LIBERO episode results;
  preserve commands, logs, checkpoint identity and output evidence.
- [ ] Two managed runs have fresh workload and monitor instances, readable CPU
  MCAPs and captured evaluation results. Verify lower directories during execution
  and the selected output/cache disposition after shutdown.
- [ ] Verify `runtime` exit code 0 is accepted while the policy server remains
  running until cooperative stop; a nonzero evaluator exit records a failed run.
  Verify the bounded evaluation fits the recorded readiness budget, and exercise
  timeout cleanup. If readiness and completion are separated, verify an evaluation
  can outlast the startup budget without a false startup failure.
- [ ] Exercise interrupted startup, evaluator failure, manual stop and unresolved
  cleanup. Report incomplete evidence; block conflicting replacement until verified.
- [ ] Add a service to the selected upstream Compose project and verify discovery,
  observation and cleanup with unchanged workflow code and step registry.
- [ ] Verify owned processes, containers, GPU users and mounts are cleaned up;
  unrelated host resources remain intact. Preserve failed-run evidence.
- [ ] Record simulator-only scope, deviations from upstream, unqualified host
  configurations and limitations. Do not claim message recording or physical-robot
  qualification from CPU MCAP or simulated episodes.

## Integration boundary

Preparation may use an optional agent to explain or propose inputs. Persist its
choices as ordinary reviewed preparation data. Keep filesystem privileges, Docker
execution and cleanup in the trusted server. No generated workflow code or pipeline
YAML is needed. Multi-host execution remains future work.

Related: [Compose workflow](../../../../libs/workload-docker/README.md),
[filesystem isolation](../../../../libs/workload-docker/docs/102626-docker-experiment-isolation.md),
[adoption candidates](../../../../libs/workload-lifecycle/docs/102626-adoption-candidates.md).
