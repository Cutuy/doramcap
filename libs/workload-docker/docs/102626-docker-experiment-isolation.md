![Serial experiment isolation](102626-docker-experiment-isolation.svg)

# Docker experiment isolation

Created: 2026-10-26. Implementation updated: 2026-10-27.
Local Linux directory-bind overlays are implemented; qualification is recorded below.
Scope: preventing unintended state carryover between serial experiments.

[Direct upper write-back](102726-direct-overlay-writeback.md) uses rsync after
strict unmount. Advanced metadata is unqualified and ambiguous hardlinks fail.

## Completion audit — 2026-10-27

The original design (9acf3fa) was followed by metadata collection (dc31060), not
mount preparation. M3a/M3b evidence was mistakenly treated as sufficient progress
while M2/M3/M4/M5 remained open. Subsequent demo and scan-limit work exercised
passthrough binds only. This completion keeps isolation acceptance separate from
diff/observability acceptance; no milestone is complete without its native tests.

## M1 — Mount planning and preparation

### Work items

- [x] Inventory writable directory binds across projects; share overlapping trees.
- [x] Create fresh upper/work/merged paths and rewrite generated execution manifests.
- [x] Verify Docker mount-namespace compatibility; reject unsupported sources before launch.
- [x] Persist original/effective mount mappings and unwind partial preparation.

### Validation

- [x] Real Docker writes affect merged views while original host bytes remain unchanged.
- [x] Shared/overlapping paths preserve aliases and read-only access; source files stay unchanged.
- [x] Preparation failure leaves no workload running and never falls back to passthrough.

Evidence: x86_64 native `overlay` tests (3 passed); Docker suite run
`/tmp/doramcap-overlay-e2e-102727-6` (8 scenarios passed). Docker writes are audited
before harvest; nested read-only binds reject writes. Generated manifests are
validated before launching any service. Native failures verify preparation rollback.

## M2 — Harvest and teardown

### Work items

- [x] Stage changed contents after writers stop; unmount before original-tree write-back.
- [x] Preserve deletions, directory replacement and supported metadata; offer retained output.
- [x] Block further admission on incomplete harvest/unmount and preserve recovery evidence.

### Validation

- [x] Two serial runs verify default write-back and retained-output isolation.
- [x] Whiteouts, opaque directories, renames, symlinks, hardlinks and sparse files are qualified.
- [x] Busy unmount, source conflict and partial preparation failure preserve evidence and block reuse.

Historical staged-backend evidence: native tests covered retained/write-back/retained serial runs, deletions,
opaque replacement, rename, modes, user xattrs, symlinks, hardlinks and sparse files.
Busy-unmount and lower-conflict tests preserve original data; Docker busy-unmount
kept staged results and blocked admission. Current direct-backend evidence is in
the [MVP plan](102726-direct-overlay-writeback.md). Partial-apply recovery is
manual and is not claimed atomic.

## M3 — Workflow and observability

### Work items

- [x] Wire preparation/finalization steps around launch and stop/collect/remove.
- [x] Keep host-bind/container-layer/volume categories and original source identities.
- [x] Show original/effective launch changes, isolation state and harvest outcome.

### Validation

- [x] Native MCAP and existing failure cleanup tests pass with enforced overlays.
- [ ] ARM64 native overlay tests pass: blocked by the worker sudo allowlist.
- [ ] Standard ARM64 suite completes on final source: job 162 outcome unverified
  after SSH became unreachable.
- [x] Local strict Clippy, core tests and frontend production build pass.

Evidence: all 8 local Docker scenarios retain readable native BPF MCAP recordings.
Live demo validation also passed normal/failure runs, streaming, reconnect and
disconnected-viewer cleanup with both overlay steps visible. Three storage
categories retain separate manifests. `PrepareOverlays` and
`FinalizeOverlays` emit ordinary step telemetry; the demo reports finalization.
Resolution JSON records restart-policy overrides; `overlays/prepared.json` records
original/effective bind sources. Source Compose files are untouched.

Existing evidence: category-preserving metadata, quiesce/remove split and streaming
readers are implemented. Overlay validation is recorded separately above and in the
[portable validation record](102726-overlay-validation.json).

## Policy

Each experiment owns fresh processes, first-party monitors, recording sessions
and eligible writable filesystem views. Only orchestration and its self-monitor
persist. Persistent services require explicit exceptions.

Default harvest policy is **write back**: apply additions, modifications and
deletions to the original host directory after writers stop. The next experiment
intentionally inherits these changes. This preserves final persistent-file behavior;
host processes see writes only at harvest time.

Optional **retain per experiment** keeps the original unchanged and materializes
the full merged view under its artifact directory before unmount. This deliberately
copies unchanged files too, so retained results remain independent of later source
changes. OverlayFS alone does not freeze its lower tree.

Explicit per-mount exceptions remain future work; unsupported writable sources fail closed. The procedural Rust workflow consumes
prepared inputs and resolved Compose mounts; service additions require no new steps.

## Cooperation and compatibility

Runs are serial. Users and external services must avoid interfering with owned
processes, paths, mount sources and machine settings during the run and harvest.
Unrelated host activity is allowed. This cooperative boundary is intended to prevent
accidental carryover; isolation claims cover only resources owned or explicitly
qualified by doramcap.

Default adoption keeps application paths, initial files and final host-visible
results intact, with no per-service output declarations. Write-back intentionally
carries mutations into the next run; retained-output mode supplies a clean baseline.
Overlay writes become visible to external host readers only at harvest. Workloads
requiring live host-side access need a shared managed view or explicit passthrough.
Do not silently switch them to delayed visibility or advertise full transparency.

## Storage cases and result access

| Write location | Default result handling | Lifetime / exception |
| --- | --- | --- |
| Writable host directory behind our overlay | Apply additions, changes and deletions back to its original path; index that path | Unmount, validate upper and write back directly; subsequent runs can change results |
| Explicit host passthrough | Writes already reach the original path; index it without copying | No filesystem isolation for this path |
| Read-only bind | Preserve access flags; redirect only when sharing a writable tree | An ancestor/child alias sees the same managed view |
| Container writable layer | Record Docker filesystem diff (added/changed/deleted paths); no content export by default | Collect before removal; removal discards layer contents, matching ordinary disposable-container behavior |
| Named volume | Preserve existing lifecycle/removal policy; collect metadata changes where accessible and warn about persistent state/coverage | No automatic content snapshot, export, reset or new deletion policy; external volumes remain external |
| Existing per-run output directory | Index in place; avoid duplicate copies | Native MCAP output already uses this pattern |
| tmpfs / private shared memory | Preserve declared volatile semantics | Stops can destroy data; persistence requires an explicit pre-stop export hook |
| Devices, sockets and pseudo-filesystems | Record their role/exceptions | No generic filesystem result extraction |

One run artifact index maps project/service/container and container path to the
host result path, container diff manifest or retained volume identity. Include capture
status, deletions, exclusions and errors. Host-path/volume references are mutable;
label them accordingly. Optional per-run retention provides stable host-directory result files. Container
layer diffs contain paths/status only: they do not preserve file contents, prove
content identity or form a replayable delta. Users needing those files should bind
an output directory or explicitly request a future export policy.
A volume reference provides access via a mounted reader container; it is not an
ordinary host artifact directory. Exporting selected volume contents is optional.

Docker's own copy-on-write layer covers image-backed paths only. Our host-directory
overlays are separate Linux mounts supplied to Docker as bind sources. Docker APIs
provide container changes; do not inspect/manipulate Docker's
private storage-driver directories. `docker diff` gives changed paths, not contents
or an event history. Record the diff after stop and before removal. This is
filesystem-diff logging; stdout/stderr capture is a separate policy. Mounted
destinations fall outside that diff and are accounted for separately.

Named-volume warnings distinguish fresh run-owned volumes from external/reused
volumes: retained data can outlive a run, while an external/reused volume can also
feed prior state into this run. A warning does not prove isolation or reset state.
Emit it once per resolved volume, without a per-run confirmation prompt.

## Common metadata-diff step

Add `CollectFilesystemChanges` as a concrete Docker workflow step. It consumes
resolved storage identities, available pre-launch metadata manifests and stopped
container IDs. It writes a normalized change manifest and returns its path plus
counts/coverage; large file lists stay out of step inputs, outputs and telemetry.
The metadata collection supplies evidence and does not change storage disposition.

| Source | Change evidence |
| --- | --- |
| Host-directory overlay | Overlay delta interpreted with deletion/opaque-directory semantics; available file metadata |
| Container writable layer | Engine filesystem diff before removal; added/changed/deleted paths, without file content |
| Named volume | Incremental before/after metadata traversal through supported read-only volume access; no Engine file-diff API |

For each entry record project/service associations, storage identity, relative
path, change kind, and available before/after metadata (type, size, timestamps,
mode/ownership and symlink target). Record absent metadata explicitly; Engine diff
alone does not provide all of these fields. No content hashing or content export.
Do not follow symlinks out of a scanned tree or scan device/pseudo-filesystem data.
Shared storage is scanned once, with all consuming workloads referenced.

Named volumes require a baseline before workloads start. Read them through a
qualified volume-access mechanism rather than assuming a host mountpoint is
available. Unsupported drivers, inaccessible paths and incomplete
baselines produce explicit coverage warnings; absence of evidence is not an empty
diff. External writers remain subject to the cooperative-access assumption.

Default delivery: save the manifest and emit a warning summary when changes or
coverage gaps exist. Include storage kind, counts and artifact path; do not dump
unbounded file lists or contents into logs. The warning is informational and does
not convert expected file writes into workflow failure. Operational failures in
required harvesting/cleanup remain separate errors with the preservation policy
below. A source that was explicitly accepted as unobservable reports that exception.

Later, an optional injected observer/hook can receive the manifest reference and
stream entries. Keep callbacks out of serialized run configuration. Hook-driven
content retrieval is a separate opt-in capability, with access/retention lifetime
specified before resource removal. No hook framework is required for this milestone.

## Performance and low ceremony

- Resolve all selected Compose projects automatically; adding a service requires no
  workflow changes or output registration. Overlay eligible writable directory binds
  only. Preserve access flags and intentional sharing.
- No upfront workspace copy, full-image export or default content hashing. Large
  unchanged models stay in the lower tree. Whole-file copy-up on modification remains
  a measured cost, not a promise of block-level copy-on-write.
- Harvest host-overlay changes and metadata. Container layers incur a diff query
  and manifest write only; do not recursively export changed directories or copy
  large files from the container. Reject ambiguous hardlink write-back and deduplicate overlapping selections.
- Stream host inventories and file payloads with sequential async traversal. Metadata
  indexes use dynamic memory proportional to tree size; there are no entry cutoffs.
  Blocking filesystem work must not stall the async orchestration loop. Target hosts include 4 GB machines.
- Keep original host output locations by default. Dedicated fresh host output
  directories are an opt-in policy, except for our own first-party artifacts.
- Write-back copies selected upper data directly; rsync logs transfer statistics and operations.
  Dedicated timing, disk-space and peak-memory measurements remain a qualification task. Sparse files and
  large changed files need qualification. No silent success on disk exhaustion.
- Container contents are disposable under the default policy; retain their diff
  evidence only. If diff collection itself fails, report incomplete evidence and
  preserve the stopped container for resolution. Do not claim result-file retention.
  Bound metadata processing and qualify Engine diff-response size on large trees.
- No new prompts for each run or mount. Resolve unsupported semantics during setup
  or fail preflight with the exact path and required exception.

## Filesystem mechanism

![Host overlay mounts](102726-overlay-mounts.svg)

For each eligible directory tree, use the original as an unchanged lower source, a fresh upper
and work directory, and a merged mount. Bind the merged path into containers at the
original destination. Upper and work reside on the same supported filesystem;
work starts empty. Keep all experiment artifacts outside the lower tree.

| Resolved mount | Proposed treatment |
| --- | --- |
| Writable ordinary directory | Experiment overlay; write-back default |
| Read-only directory | Preserve read-only bind; record identity |
| Same source across services/projects | Share one merged view per experiment |
| Parent/child sources | Plan one compatible overlay tree; preserve each target's access mode |
| Conflicting policies on shared/overlapping sources | Reject before launch; resolve during onboarding |
| Dedicated outputs already configured by user | Keep original destination; fresh per-run directory only when selected |
| Single-file bind | Initial explicit exception; parent-overlay support needs alias/permission qualification |
| Named volume | Separate policy: fresh experiment volume; external/reused volumes are declared carryover |
| Device, socket, procfs or sysfs | Preserve required semantics and record exception; exclude from overlay traversal |

Use Compose's normalized model for paths, overrides, profiles and interpolation.
Canonicalize source aliases and inspect file types before planning. Nested mounts,
symlinks, hardlinks across roots and unsupported backing filesystems need explicit
qualification; never silently split a shared workspace or claim full isolation.
Read-only access through one container path does not prevent writes through another
alias or from the host.

Overlay reads reuse lower files. Writes may copy an entire existing file into the
upper layer, including a large model. Prefer read-only model/dataset mounts where
known. The lower tree must remain stable while overlays are mounted; external host
writers are incompatible with that assumption. Avoid modifying Docker's own storage
layers: these are separately managed host overlays exposed through bind mounts.

## Workflow and step boundaries

Keep the workflow ordinary async Rust. The preparation step receives the Docker adapter for validation. Linux mount
operations use the existing libc dependency; installed rsync handles file transfer.
Inputs/outputs remain serializable paths, policies and resolved project records. No pipeline declaration format or orchestration context is introduced.

1. Resolve Compose and inventory all projects' mounts together.
2. **Prepare overlays:** validate policies, prepare mounts, persist ownership records,
   capture required metadata baselines, and generate execution manifests with
   substituted source paths.
3. Start fresh first-party services, then foreign projects; observe the experiment.
4. Stop foreign projects, then first-party services; verify all writers have exited.
5. **Collect filesystem changes:** persist normalized metadata diffs and emit warning
   summaries. Keep stopped containers until required diff logging succeeds.
6. **Finalize filesystem results:** unmount overlays and verify absence before changing
   their lower trees; validate upper and apply direct write-back or finalize retained artifacts.
7. **Remove runtime resources:** remove stopped containers (discarding their writable layers)
   and apply existing volume/scratch cleanup policy after disposition is recorded.
8. Admit the next experiment only when cleanup and harvest disposition are resolved.

First-party MCAP/artifact output paths use dedicated experiment directories.
Harvest runs after the recorders finalize; lifecycle telemetry covers these steps.
Quiesce and removal are separate steps around collection/finalization. Compose down can delete containers and selected volumes; it
belongs after diff logging and filesystem finalization. tmpfs export, when requested,
must occur before stop.

Do not modify lower while any overlay view is mounted. Strictly unmount, validate
all upper trees, interpret whiteouts and opaque directories, then apply selected
upper entries. An upper delta is not an ordinary directory for recursive copying.

## Harvest semantics and failure handling

- Inspect upper after unmount. Interpret whiteouts and opaque directories and reject
  unsupported redirect/metacopy and ambiguous hardlinks before modifying lower.
- Apply deletions as well as writes. Rsync archive mode preserves basic attributes
  and symlinks; ACL, xattr, capability and hardlink fidelity are unqualified.
- Use incremental traversal with bounded concurrency and full metadata manifests; no default content hashing or arbitrary scan cutoffs.
  Record unreadable paths, exclusions and incomplete scans. Metadata checks cannot
  prove content identity or eliminate concurrent-write races.
- Check the original against captured baseline metadata before write-back. Detected
  conflicts stop automatic write-back and preserve evidence; stable/exclusive source
  access remains the precondition for reliable application.
- Write-back across a directory tree is not atomic. Record per-operation outcomes;
  preserve upper data on partial failure and require resolution before another run.
- A workload failure still reaches harvest after verified shutdown, preserving normal
  write-back semantics. Failed preparation unwinds only resources already owned.
- If writers cannot be stopped, skip harvest and retain resources for resolution.
  Failed unmount blocks admission. Do not use lazy unmount as proof of cleanup.
- Failed container diff collection preserves stopped containers and blocks automatic
  removal pending resolution. A successful diff permits removal without file export.
  Retained volume identities remain accessible; explicit volume removal keeps its
  existing discard semantics and is recorded in the outcome.
- Retained results survive cleanup. Remove an upper directory only after successful
  write-back or verified materialization and the selected retention policy permits it.

No automatic crash-resume mechanism is planned. Manual recovery may be necessary
for abandoned mounts; a fresh manager must refuse collisions with existing resources.

## Lifecycle mechanisms: process and IPC boundaries

The lifecycle contract must state what each mechanism actually isolates. All need
recording drain/finalization and verification that owned work has ended.

| Mechanism | Process boundary | IPC/filesystem boundary |
| --- | --- | --- |
| Docker | Container process namespace and cgroup; verify stopped containers and restart policy | Private IPC and `/dev/shm` normally help; host/shared IPC and binds are exceptions; volumes persist |
| systemd units | Per-unit cgroup; qualify `KillMode`, delegated children and stop completion | Optional `PrivateIPC`, `PrivateTmp`, mount restrictions and runtime directories; inspect configured settings rather than assuming them |
| supervisord | Supervises configured processes; `stopasgroup`/`killasgroup` extend signaling | Host namespaces by default; shared memory/files persist; descendants that escape groups require stronger ownership |
| Explicit subprocesses | Parent wait and optional process-group signals | Host state shared by default; cgroups/namespaces and owned runtime directories must be provided explicitly |

System V IPC namespaces and filesystem-backed POSIX shared memory are different
boundaries: `PrivateIPC` alone does not establish a private `/dev/shm` filesystem.
Process groups alone cannot contain children that create new sessions. An adapter
must record unsupported cleanup coverage instead of inheriting Docker's guarantees.
No generic cleanup should delete unrelated host IPC objects.

## Other isolation domains to explore

Serial execution removes concurrent experiments, but residual state still matters.

| Priority / domain | Pollution examples | Boundary to qualify |
| --- | --- | --- |
| P0: process ownership | Escaped children, host launchers, restart loops | Stop owned process trees/cgroups; verify emptiness and disabled restart |
| P0: filesystem beyond binds | Named volumes, container layers, temp files, build/model caches | Fresh storage by default; explicit carryover and output retention |
| P0: IPC and middleware | Host shared memory, semaphores, ROS/DDS participants, retained broker messages | Private IPC where possible; fresh domains/services or scoped draining/reset |
| P0: recorder/session state | Buffered messages, stale subscriptions, reused counters | Fresh recorders, drain/finalize, associate every stream with the current run |
| P1: networking | Host listeners, bridges, firewall rules, external connections | Owned network teardown; baseline/delta checks for host-network exceptions |
| P1: host configuration | CPU affinity/governor, cgroups, sysctls, GPU clocks/power settings | Record before; restore only owned changes; verify postconditions |
| P1: devices and robot state | GPU contexts, camera settings, controller modes, pose, simulator state | Workload-specific stop/reset/readiness hooks; require readiness before next run |
| P1: external services | Remote robot servers, databases, queues, object-store writes | Run namespaces or explicit reset contracts; track persistent exceptions |
| P2: measurement conditions | Warm caches, thermal throttling, GPU allocator/compiler caches | Observe and declare warm/cold policy; cooldown gates when needed |

Host IPC/network and privileged containers weaken automatic cleanup guarantees.
Prefer private namespaces when compatible; preserve required robotics semantics
and expose exceptions. Do not delete global IPC objects, flush host caches or reset
shared devices indiscriminately. Physical robot state requires application-specific
contracts; container shutdown alone cannot establish it.

Suggested sequence: filesystem overlays → process/IPC/recording qualification →
host configuration restoration → device/external-service hooks → measurement baselines.

## M4 — Further filesystem and hardware qualification

### Work items

- [ ] Measure large-model copy-up, staging cost, peak metadata memory and disk usage.
- [ ] Qualify additional backing filesystems and rootless/private-namespace daemons.
- [ ] Add explicit policies for single-file/device binds and live host readers.
- [ ] Qualify vEDGAR/AnywhereVLA workloads on their required GPU/robot hardware.

### Validation

- [ ] Exercise disk exhaustion and partial write-back; verify retained data and journals.
- [ ] Record large-model benchmark evidence without conflating metadata scans with copying.
- [ ] Prove unsupported semantics fail visibly on each additional host configuration.
- [ ] Repeat real robotics runs with documented readiness/reset contracts.

## References

- [Linux OverlayFS semantics and backing-filesystem requirements](https://www.kernel.org/doc/html/latest/filesystems/overlayfs.html)
- [Docker overlay storage and copy-up behavior](https://docs.docker.com/engine/storage/drivers/overlayfs-driver/)
- [Docker bind mounts](https://docs.docker.com/engine/storage/bind-mounts/)
- [Docker workflow](../README.md)

- [Docker container changes](https://docs.docker.com/reference/cli/docker/container/diff/)
- [systemd execution environment](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html)
- [Supervisor process-group settings](https://supervisord.org/configuration.html)

## Design update evidence

2026-10-26: expanded storage cases, result access, cooperative host assumptions,
backend comparison and performance gates. Updated the diagram to separate capture,
unmount/write-back and resource removal. Implementation and dependency selection
remain pending; rustix/rsync have not been adopted.

Default-policy clarification, 2026-10-26: host overlays write back; container layers
log filesystem diffs with no file export; named volumes preserve existing policy
and report carryover warnings. This supersedes the earlier automatic container
file-export proposal. Local links, SVG XML and whitespace checks passed.

Metadata delivery update, 2026-10-26: a common collection step persists normalized
file-change metadata and logs bounded warning summaries by default. Named-volume
coverage requires a supported before/after scan. Optional injected hooks and
content delivery remain future work; no new runtime implementation is claimed.

M3a/M3b implementation evidence: [metadata collection milestones](102626-filesystem-metadata.md)
and [validation record](102626-filesystem-validation.json). Seven Docker scenarios
pass; that historical evidence covered passthrough metadata only. The 2026-10-27
completion evidence above covers actual mounting and post-unmount write-back.

## Supported host contract

The manager and local Docker socket peer must share a mount namespace. Mounts
require CAP_SYS_ADMIN and an OverlayFS-compatible upper/work filesystem. Host
source trees remain stable under the cooperative-access assumption. Rootless,
private-namespace and proxied daemons require further qualification.

Preparation rejects writable single files, sockets/devices, pseudo-filesystems,
nested mounts, cross-root hardlinks, overlapping artifact directories and
unqualified propagation/SELinux settings. Upper/work directories are fresh.
Sources must already exist. Ordinary read-only inputs stay direct; shared read-only
aliases retain their flags. With `redirect_dir=off`, renaming lower directories may
return EXDEV; applications need their usual copy/delete fallback. Whole-file
copy-up can affect large mutable models.
No failure falls back to a writable host bind.

`overlays/plan.json` and `prepared.json` identify ownership. Per-tree `baseline.pb`,
`changed.paths0`, `apply-plan.pb`, rsync logs and `apply.pb` retain evidence.
`unmounted.json` precedes lower modification; `results.json` records successful
disposition. Failed/cancelled calls leave evidence for manual resolution; no crash
recovery or automatic retry is implied. Upper and staging data are retained.

ARM validation, 2026-10-27: jobs 157/158 passed compilation, strict lint,
default/all-feature tests and the existing native suite, then failed when direct
sudo execution of the new overlay test required a password. Further attempts
stopped. The unsupported CI invocation was removed. Worker provisioning must
extend its approved root-owned native-test launcher before ARM mount validation;
no sudo policy or persistent worker configuration was changed.

Standard-suite job 162 used snapshot `542d8a0bb6914550b423e531a190ef1d`. It was
last observed running; subsequent SSH connections timed out. Its final result
remains unverified. Local native tests were rerun after the final behavior change.
