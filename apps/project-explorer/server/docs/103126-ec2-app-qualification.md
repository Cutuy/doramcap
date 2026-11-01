![App qualification architecture](103126-ec2-app-qualification.svg)

# EC2 CPU app qualification

Extend the existing private ARM64 gallery beyond Activity Lab. Start from remote
main `79d86d56044943f8ca1e9129ffd8d90e668daa20`, build on the ARM worker, and
qualify each app on the 1 GiB EC2 host. Live addresses and credentials stay
outside Git. Keep CUDA disabled; Orchestrator Lab needs its separate host worker.

## M1 — Build CPU app images

**Work items**

- [x] Acquire a lease based on current remote main and inspect wrappers.
- [x] Separate Fault Watchdog's host dashboard port from Fault Demo.
- [x] Build native Storage, Dataflow, Fault, Monitor, Recorder and CPU Viewer inputs.
- [x] Build frontends and package images, deployment files and checksums.

**Validation**

- [x] Record the exact source revision and ARM build job results.
- [x] Verify image architecture, checksum manifest and focused frontend tests.

Evidence: ARM worker has Docker, Node/npm, Cargo and Dora 1.1.0. Its default
queue is paused with no running jobs and about 4.5 GiB disk free. Existing caches
are preserved. Source commit `24127abbd6afd5811d6311b6a78e669ad0ddfb6f`
changes Fault Watchdog's existing listener/registration to port 8089; Fault Demo
keeps 8088. Alternate MCAP/OTLP registrations also select 8089 because they
inherit that dashboard listener; those profiles are outside this default-profile
deployment. Build/image job 554 passed in the serial default queue. Native release builds
completed in 7m19s; frontend builds and tests passed: Storage 12, Dataflow 6,
CPU Viewer 12, plus the Storage frontend contract check. All seven images report
`linux/arm64`. The image/runtime archive is 228 MiB; SHA256
`45d57e1f73420d69ef2cd709dfb0952e6ef543682319e8ca18972e29ec1b3f7b`.
Worker per-file checksum verification passed. Only this chat's redundant
intermediate release archives were removed to free space; shared CI caches remain.

## M2 — Qualify portable CPU apps

**Work items**

- [x] Install images and runtime files while preserving the stable host workspace.
- [x] Enable Storage, Dataflow Tracing, Fault Demo and Fault Watchdog.
- [x] Preserve the private bridge origin and proxy configuration.

**Validation**

- [x] Launch each app, verify readiness and routed APIs/streams or controls.
- [ ] Verify shared sessions, final artifacts and cleanup after session expiry.
- [ ] Check independent generations, selected concurrency and available memory.

Evidence: archive and per-file checksums passed on EC2. All four apps reached
readiness and passed routed checks. Storage produced binary snapshot/activity
streams and a valid 1,622,725-byte live MCAP capture after managed/slab writes
and rotation. Dataflow reported four nodes and completed work. Both Fault apps
reported five conditions and accepted fail/healthy controls concurrently.
The existing Dataflow live decoder test also passed from the ARM worker against
EC2: four node identities, 649 aligned step/activity pairs, lineage links and
reconnect deduplication. All four apps passed independent first-viewer expiry
while a second viewer kept each generation ready. Final expiry is being checked.
With all four running, about 338 MiB remained available; swap use was about
219 MiB. Manager peak memory was about 96 MiB with no restarts.

## M3 — Qualify native monitoring

**Work items**

- [x] Check EC2 kernel BTF, cgroups and BPF capabilities.
- [ ] Qualify System Monitor, MCAP Recorder and CPU Viewer individually.
- [ ] Keep unsupported apps disabled and record concrete blockers.

**Validation**

- [ ] Verify real telemetry, delivery counts and finalized artifacts.
- [ ] Verify bounded memory and cleanup; distinguish ARM builds from EC2 runtime.
- [ ] Review public changes for credentials and private connection details.

Kernel inspection passed: Linux 7.0.0-1014-aws, readable BTF, cgroup v2, and
`kernel.io_uring_disabled=0`. Unprivileged BPF is disabled and perf restrictions
are enabled; qualification uses only the capabilities in the existing app recipes.
No persistent kernel security settings were changed. The real attachment probe
reported `attachment_verified=true`. A separate task-accounting run at capacity
32768 completed with `collected=2 published=2 dropped=0`.

Full workspace CI, GPU qualification and automatic recording retention are
outside this pass. Record focused checks and deferred coverage beside each result.
