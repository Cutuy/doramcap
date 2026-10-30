![Console responsibilities](102926-console-boundary.svg)

# Console MVP — engineering plan

Created: 2026-10-29. Status: implementation pending. Paths marked **new** below
are planned deliverables. Existing symbols and gaps were checked against PR #10's
Rust source; unchecked boxes describe work still to do.

One page: **project directory + optional profiles → Review → Start → workflow
steps → Stop → current result/downloads**. One user, one Linux host, one run.
Use local prebuilt images and the existing CPU monitor. First qualification uses
a small Compose fixture; vEDGAR `singlepc` follows separately.

Defer custom file/env pickers, individual service selection, history browsing,
event replay, general-purpose charts, workflow editing, builds/pulls and automatic crash recovery.
Keep the existing filesystem policy and capture limitations; no new isolation or
provenance subsystem belongs to this plan.

## UI language and concept renderings

These are implementation references for the same page at different moments.
Names, durations and run IDs are illustrative. The phase rail shows location in
that flow; it is not navigation between independent screens. Initial delivery uses
this light theme. Shared components and tokens own the appearance of every state.

### Shared rules

| Element | Rule |
|---|---|
| Page | Quiet background `#F5F7F6`; white surfaces; border `#D9E2DD`. Center the content with 64 px side margins at 1200 px. |
| Text | Primary `#20352F`, secondary `#596B64`. System sans; 30 px page title, 19 px section title, 15 px body, 12–14 px labels. Monospace only for paths/identifiers. |
| Spacing | 4 px base; use 8/16/24/32 px spacing, 12 px panel corners and 8 px control corners. Inputs/buttons are at least 44 px tall. |
| Actions | Teal `#166653` with white text for the primary action. Secondary actions are outlined. Stop uses an amber label and remains visually separate from a failure. |
| States | Active teal; completed `#267047`; attention `#8B5A13` on `#FFF3DC`; failed `#A33636` on `#FBECEC`; unknown neutral. Always include a text label. |
| Layout | Main content takes about two thirds of the width; context takes one third. At widths below 900 px, stack context beneath the main content and wrap long paths/step labels. |
| Accessibility | Visible labels, semantic controls and tables, 2 px focus outline with 2 px offset, logical keyboard order and minimum 4.5:1 contrast for normal text. Announce lifecycle changes politely; do not announce every poll. |
| Motion | No completion percentage or invented future steps. Running is a labeled state; any optional motion respects reduced-motion preferences. |

Use the same vocabulary everywhere: **Project, Review, Run, Result** for page
phases; **Review project, Start run, Stop run, New run** for actions. Workflow
states are **Running, Completed, Failed, Cancelled, Panicked**. Results always
separate **Execution, Recording, Cleanup**; an unavailable outcome is **Unknown**.
Friendly labels such as “Prepare project” map to known emitted step identities;
retain the full emitted identity in accessible row details. Unknown steps use
their emitted names. Repeated invocations remain separate rows.

### Project and review

The project concept shows profiles after discovery. Before the first review, hide
that control; when no profiles exist, omit it. Changing the directory/profile
invalidates the review. Missing prerequisites use an inline explanation and disable
Start. Review shows the resolved application with read-only service rows.

![Project selection concept with optional profile](102926-console-ui-project.svg)

![Read-only project review concept](102926-console-ui-review.svg)

### Run

Keep the project/run identity visible. Show received workflow steps and completed
durations, including repeated observations. The sample rows are illustrative;
implementation displays actual emitted invocations. CPU recording becomes “In
progress” only after monitor readiness. A lost connection says “Connection lost;
run may still be active” and retains the last snapshot with its age.

Stop changes the heading to “Stopping run”, disables repeated clicks with the
label “Stopping…”, and continues polling through finalization and cleanup. A
telemetry gap adds “Progress incomplete”; history eviction says “Older steps
hidden”. Neither message changes the authoritative run outcome.

![Workflow progress concept](102926-console-ui-run.svg)

### Result and blocked cleanup

Downloads are the primary result action. New run is secondary and is enabled only
after verified cleanup. The blocked concept deliberately preserves the saved
recording: recording validity and cleanup are independent. Missing artifacts show
an unavailable label with a reason. Do not imply recovery through a Retry button.
After process loss, show “Interrupted” and mark unverified outcomes Unknown.

![Completed run and downloads concept](102926-console-ui-result.svg)

![Saved recording with unverified cleanup concept](102926-console-ui-blocked.svg)

## Existing code and required changes

| Existing code | Reuse / engineering gap |
|---|---|
| [ComposeProject](../../libs/workload-docker/src/compose/model.rs) | Requires explicit `files`; discovery must populate them. Empty `services` selects the enabled application. |
| [DockerAdapter::resolve](../../libs/workload-docker/src/compose/adapter.rs) | Resolves/pins images and writes a private manifest with `sha256`. Automatically loading `.env` is disabled. |
| [Compose command helper](../../libs/workload-docker/src/compose/command.rs) | Reuse its socket, clean environment, argument handling, output limits and timeouts for profile discovery. |
| [ComposeRunner::run_with_ready](../../libs/workload-docker/src/workflows/recorded_compose/mod.rs) | Calls `PrepareRun` internally, then mounts/launches. Add a review check immediately between those operations. |
| [TelemetryObserver](../../libs/orchestrator/src/telemetry.rs) / [workflow-protocol](../../libs/telemetry/workflow-protocol/proto/workflow.proto) | Existing step events and codec; Console supplies transport and a bounded reducer. Observation repeats every 500 ms. |
| [ComposeRunReport](../../libs/workload-docker/src/workflows/recorded_compose/mod.rs) / [system apps](../../libs/workload-docker/src/workflows/system_apps.rs) | Already produce outcome fields, `report.json` and `cpu.mcap`; no new recorder. |

## Deliverables and dependency order

| Milestone | Code delivered | Depends on |
|---|---|---|
| M1 | Addon loads; generated contracts; one Rust owner thread | — |
| M2 | Directory/profile preparation and a review guard before launch | M1 |
| M3 | Start/Stop, bounded workflow progress and durable admission | M2 |
| M4 | One-page HTTP/UI flow and current-result downloads | M3 |
| M5 | Packaged launch and manual Docker qualification | M4 |

Implement each milestone as a reviewable change with its tests. Record commands,
revision and observed results beside its validation items before checking them.

![Native library interop](102926-console-interop.svg)

## M1 — Establish the native owner and contracts

**Work items**

- [x] Create **`console/server/native/`**, Cargo package `console-native` (`cdylib`
  plus `rlib` for tests), and add it to workspace/boundary rules. New modules:
  `lib.rs` (bindings), `runtime.rs` (owner loop), `config.rs` (pbtxt loading).
- [x] Evaluate [napi-rs](https://napi.rs/) against [Neon](https://neon-rs.dev/)
  for Promise completion, owned buffers and Linux packaging; present the decision
  before adding crates. Prefer napi-rs. Reuse existing Tokio, prost, prost-reflect,
  sha2 and rustix; use Node's HTTP/filesystem modules and the existing React/Vite/
  protobufjs toolchain. Pin additions after evaluation; no binding is adopted here.
- [x] Add **`console/proto/console.proto`** and generate Rust/TypeScript codecs.
  Define `ConsoleConfig`, prepare/start requests, review, status, error and marker
  records. Import the existing system-app and workflow event schemas; configure
  prost external type mappings to reuse their generated Rust types. Protobuf
  uint64 values remain decimal strings/Long in TypeScript, including IDs and time.
- [x] `ConsoleConfig` fields: listen address, approved project root, data root,
  Docker socket, access-secret file, public Origin and `SystemAppsConfig`. The public Origin is explicit because a wildcard listen address cannot identify the remote browser Origin. Expose configured CPU recording as an optional review-time system app (M12);
  reject configured Dora flows for this first slice. Parse pbtxt using the existing
  prost-reflect pattern in [registry.rs](../../apps/project-explorer/server/src/registry.rs).
  Use generated configuration types directly.
- [x] Implement addon `open(configPath)` returning a Promise of a Console handle.
  Spawn one Rust thread with a current-thread Tokio runtime and LocalSet; construct
  `Rc<DockerAdapter>` and `ComposeRunner` there. A bounded command mailbox (32)
  carries owned requests with one-shot replies. Full mailbox returns Busy; no
  JavaScript values or `Rc` objects cross threads.
- [x] Expose async `prepare`, `start`, `status`, `stop`, `completed` and `close`
  methods. Encode records as owned protobuf buffers across N-API. Include a
  contract version in initialization and reject a server/addon mismatch before
  listening. Add server package scripts for codec generation, native build, TypeScript
  build and Node test runner; check in its lockfile. `close` signals the owner and
  awaits its exit; finalizers do not stop runs.

**Validation**

Partial evidence (2026-10-29, working tree based on `3418acc`): `cargo test --profile ci --locked -p console-native` passed five unit tests for owner thread affinity, mailbox saturation, configuration and uint64 encoding. `npm test` in `console/server` passed three tests using a local Docker API stub, including two open/close cycles and Node responsiveness during startup. `npm run build` passed. The current Node 22.23.3 server suite passes seven tests, including contract mismatch, two open/close cycles, a 100 ms HTTP heartbeat during delayed native startup, and exact sequence and invocation values above 2^53. The owner-thread and mailbox tests pass in the native suite.

- [x] **`native/tests/runtime.rs`**: injected adapter factory records construction,
  command and destruction thread IDs; all equal the owner and differ from caller.
  Fill the mailbox and assert Busy without blocking the caller.
- [x] **`server/tests/addon.test.ts`**: import the built addon, open/close twice,
  reject a version mismatch, and round-trip an invocation ID greater than `2^53`.
  Use Arrange/Act/Assert sections in Rust and TypeScript test cases.
- [x] Run a delayed native test operation while an HTTP heartbeat serves every
  100 ms; assert a response within 1 s throughout the delay. Build/test commands:
  `cargo test --profile ci --locked -p console-native` and `npm test` in `console/server`.

## M2 — Resolve the default project and guard the reviewed launch

**Work items**

- [ ] Add **`native/src/project.rs`**. Canonicalize the selected directory beneath
  the configured root. Discover only that directory: exactly one of `compose.yaml`,
  `compose.yml`, `docker-compose.yaml`, `docker-compose.yml`; reject absent or
  ambiguous bases. Accept at most one `compose.override.{yaml,yml}` for a compose
  base, or `docker-compose.override.{yaml,yml}` for a docker-compose base.
  These conservative rules avoid choosing among ambiguous entry points. Pass base
  then override explicitly; add `.env` to `env_files` if present. No ancestor search,
  shell-environment inheritance or custom file list. Repositories are trusted host
  inputs; the approved root restricts selection, not container filesystem authority.
- [ ] Add `DockerAdapter::profiles(&ComposeProject)` in
  **`libs/workload-docker/src/compose/adapter.rs`**. Factor shared config arguments
  out of `resolve`, retaining `/dev/null` plus explicit env files, approved socket
  and clean environment. Call `config --profiles` through the existing command
  helper. Validate selected profiles against this list; keep `services=[]`.
  Compose owns parsing, interpolation, includes and service `env_file` semantics.
- [ ] Add **`native/src/prepare.rs`**. Allocate a random run ID and private
  `data/reviews/<id>/`; resolve the workload with key `workload`. Reuse
  `PrepareSystemApps` and resolve its monitor project to check/pin its local image.
  Return run/prepared IDs, profiles, enabled service names/image IDs and policy
  warnings. Check the existing overlay namespace requirement. Report CPU monitor
  image availability; live capture readiness comes from `WaitForSystemApps` after
  Start. Never return resolved manifests or interpolation values.
- [ ] Keep one in-memory prepared record: original `ComposeProject`, run ID,
  workload manifest digest, resolved monitor image ID and fixed `ComposeRun`
  settings. Use the same run ID/project key at Start; replace the previous review
  on new preparation. Start uses new `data/runs/<id>/`, the existing default overlay
  policy, readiness 60 s and stop 15 s. Before dispatch, repeat discovery and reject
  changed file/env inventories (including newly added overrides). Delete obsolete
  private review directories after replacement/consumption; never delete run results.
- [ ] Add `ComposeRunner::run_with_preflight(spec, stop, observer, ready, check)` in
  **`recorded_compose/mod.rs`**. Factor the shared run body so existing `run` and
  `run_with_ready` retain their behavior. Invoke the one-shot check on `PreparedRun`
  after preparation and before `blocked=true`, `PrepareOverlays` or any launch.
  A rejected check returns an error with private preparation files only.
- [ ] Console's check compares the newly resolved workload digest and monitor image
  ID with the review. Run/project identity is identical; do not compare the monitor
  manifest digest because its output paths change between review/run directories.
  A mismatch returns StaleReview and invalidates Start. Execution consumes those
  just-checked frozen manifests. This checks effective configuration and image
  identity; mutable bind contents retain the runner's documented coverage limits.

**Validation**

Partial evidence (2026-10-29, working tree based on `3418acc`): `cargo test --profile ci --locked -p console-native` passed discovery tests for four base names, matching overrides, `.env`, ambiguous inputs and directory escapes. `cargo test --profile ci --locked -p workload-docker` passed 14 unit tests; its 17 capability/Docker tests were ignored in that unprivileged invocation. `real_compose` was built with `cargo test --profile ci --locked -p console-native --test real_compose --no-run` and its executable ran under sudo: both default versus explicit Compose/profile discovery and preflight rejection without owned resources or mounts passed. The real `changed_inputs_reject_before_launch` test also passed for a changed effective `.env` value and a newly added override, with no owned resources or mounts. The same real-Docker test passed for command and included-file changes; `retagged_local_image_rejects_before_launch` passed using a unique temporary alias that the test removed. The broader CLI equivalence cases and unchanged-review launch were subsequently verified below.

Further evidence: `default_and_explicit_discovery_match_for_supported_names` passed unprivileged with Compose v5.6.0 for all four base names, both matching override extensions, default and selected-profile services, included files, service env files and a newly added service. The packaged acceptance launched an unchanged reviewed configuration twice.

The capability-required `changed_inputs_reject_before_launch` executable also passed with a newly added `.env` alongside changed environment, override, command and include cases. Its first new-`.env` attempt failed because the fixture had an unset interpolation variable; adding an explicit Compose default corrected the fixture before the successful rerun.

The real review test passed with `VALUE=hostile` in the test process: the resolved value came from the explicit `.env`, and an undeclared profile was rejected. Project unit tests passed the base, override, missing, ambiguous and root-escape cases.

- [x] **`native/tests/project.rs`**: table cases for each base name, base+override,
  no file, ambiguous files, `.env`, invalid profile, outside-root and symlink escape.
  A hostile server environment must not alter the resolved project.
- [x] **`native/tests/prepare.rs`** with real Compose: compare enabled services and
  resolved values to both default CLI discovery and the equivalent explicit invocation
  for supported unambiguous base/override combinations,
  `.env`, included service/env files and profile cases. Add a new service and assert
  discovery includes it without changing runner code.
- [ ] **`libs/workload-docker/tests/docker.rs`**: reject the preflight callback and
  assert no new owned Docker resources or overlay mounts. Existing runner tests
  remain unchanged and pass through the default no-check path.
- [x] Add an override or `.env` after Review; Start rejects the changed input set.
  Change command/env/include contents and retag a local image after Review;
  each effective change rejects Start before launch. An unchanged review launches
  its pinned images. Run `cargo test --profile ci --locked -p workload-docker`
  plus the explicit real-Docker cases under the capability rules in M5.

## M3 — Own one run and reduce workflow telemetry

**Work items**

- [ ] Add **`native/src/session.rs`** with transitions
  `Idle → Preparing → Reviewed → Starting → Running → Stopping → Finished` and `Blocked` on
  uncertain cleanup. Allow one pending preparation; reject Prepare during a run,
  and permit Start only from Reviewed. Persist Starting before dispatch; resolve
  Start's Promise only
  after the preflight check accepts, before waiting for readiness. Resolve it with
  StaleReview on a digest mismatch. The run ID is already in the review/status so
  Stop is usable while Start awaits preparation. Keep the runner future polled on
  LocalSet while the command loop serves status and Stop. Retain the readiness
  receiver; its signal moves Starting to Running, never Stopping back to Running.
  Store a one-shot stop sender; repeated Stop succeeds. Reserve a control path for
  Stop/close outside the ordinary mailbox so admission saturation cannot starve them. Check a latched Stop in
  the preflight callback so Stop during preparation cannot lead to a later launch.
- [ ] Feed an observer wrapper into `run_with_preflight`: remove payloads and
  replace raw errors with safe error categories before forwarding to
  `TelemetryObserver`. Its sink copies encoded bytes into a 512-entry queue using
  `try_send`, with a 16 KiB event limit. Queue-full returns an error to the observer;
  the observer's loss counter remains accessible even if the last event was lost.
- [ ] Add **`native/src/progress.rs`** to decode `workflow_protocol::Event` and keep
  at most 512 invocation rows keyed by `(session, executor, invocation)`. Update
  states/durations from emitted events. Evict oldest terminal rows first; count
  history truncation separately from lost events. If all rows are active, drop new
  rows with an overflow count. Unknown event versions set progress incomplete.
- [ ] `status()` returns lifecycle, rows, sequence/loss/truncation counters and the
  final outcome. Use a short snapshot lock shared with the owner; a full event queue
  must not prevent reading status or reporting completion. Preserve terminal
  completion outside the queue. The UI displays step names and states; container
  inspection remains the runner's implementation detail.
- [ ] Add **`native/src/state.rs`**: acquire a nonblocking data-root file lock using
  existing rustix; persist `current.pb` before dispatching Start (0600 temporary
  file, sync, rename, parent sync). Marker fields: version, run ID, request key,
  phase, artifact reference, public project identity, acceptance flag and final outcome. Same-key retries share the original
  acceptance result, including while preparation is pending; a different key during
  an active/blocked run returns Conflict. A known prelaunch rejection/Stop with
  `!runner.blocked()` records no-launch, invalidates review and returns to Idle.
- [ ] On completion, require both `report.boundary_clean` and `!runner.blocked()`
  before admitting another run. A returned error or marker-write failure without
  verified cleanup enters Blocked. On startup, restore Finished or block on an
  unfinished/corrupt marker. No automatic marker clearing or unblock HTTP route.
- [ ] Implement `close`: stop admission, signal Stop, await finalization and persist
  the result, then join the owner thread off the Node event loop. After a 120 s
  process-shutdown deadline, retain the unfinished marker and exit nonzero.
  Document that forced process loss cannot guarantee cleanup; recovery follows the
  existing [isolation procedure](../../libs/workload-docker/docs/102626-docker-experiment-isolation.md).

**Validation**

Partial evidence (2026-10-29, working tree based on `3418acc`): `cargo test --profile ci --locked -p console-native` passed the bounded reducer and marker/lock unit tests. The unprivileged-built native test executable ran under sudo with `node:22-bookworm-slim` and `doramcap/system-monitor:native-cleanup-20261029` (image ID `e22bbfffd68a`): run `96d4044ada1aa549b2d8b04f0fc5b677` joined duplicate Start, rejected a second key, reached CPU readiness, stopped, finalized MCAP and verified cleanup; private evidence is `/tmp/.tmp3O3QhU`. A later run `0c68825568d296518665e9c1855cc174` verified MCAP session `1793318270577948797-1` with 49 contiguous snapshots; run `24a463759297eabf775f25202b9fe054` verified a distinct session `1793318337466856302-1` with 63. The real `stop_during_preparation_prevents_launch` test passed with no owned resources or mounts. Run `3b1639960c7c03456324232e37e227c3` forced an unclean test-owned workload termination: recording finalized, cleanup was unverified, and the next request conflicted; test-owned resources were removed afterward. An earlier recording-file removal experiment produced unavailable recording but verified cleanup, so its original expectation of blocked admission failed and was corrected. Two earlier attempts used the wrapper image `doramcap/system-monitor:local` and failed before readiness; their owned resources were checked and removed. The native reducer tests cover 10,000 completed invocations, bounded retention, terminal states, missing parents, unknown versions and final-event queue loss. State tests cover an interrupted replacement, persisted Starting before dispatch, clean completion and exclusive ownership. Node-level saturation and close-during-capture remain open.

- [x] **`native/tests/progress.rs`**: scripted Started/Succeeded/Failed/Cancelled/
  Panicked events; repeated step names with different IDs; missing parent; unknown
  version; queue-full including final-event loss. Send 10,000 completed invocations;
  assert at most 512 rows, exact truncation accounting and independently readable completion.
- [x] **`native/tests/session.rs`**: delayed preparation plus Stop produces zero
  launches; Stop while running waits for cleanup; duplicate Start launches once;
  a second key conflicts; cleanup failure keeps admission blocked.
- [x] **`native/tests/state.rs`**: crash points before marker rename, after durable
  marker but before dispatch, and before completion persistence. Restart either
  restores a proven finished result or blocks; two owners cannot acquire one root.
- [ ] **`server/tests/addon.test.ts`**: stop polling while native events saturate the
  queue, then resume; capture/Stop still complete and loss is visible. Close during
  startup and during capture; verify the owner exits only after finalization.

## M4 — Wire the single page and bounded HTTP API

**Work items**

- [ ] Add **`frontend/src/ui/tokens.css`**, `PageFrame.tsx`, `ActionButton.tsx`,
  `StatusLabel.tsx` and `OutcomeSummary.tsx` following the concepts above. Share
  spacing, colors, type and state labels; screen components do not redefine them.
  Add the friendly step-label lookup to `RunSteps.tsx`, preserving full identities.

- [ ] Add **`server/src/main.ts`**, `http.ts`, `native.ts` and `artifacts.ts`.
  Serve built frontend assets and the routes below with Node HTTP. Use generated
  protobuf JSON mappings for HTTP and binary protobuf for the addon; keep uint64
  fields as strings. Limit request bodies to 64 KiB; whitelist request fields.
- [ ] Protect all routes/assets with HTTP Basic authentication (`console` plus the
  configured secret), using Node's timing-safe comparison. Check mutation Origin
  against the configured server origin. Return structured errors: InvalidInput 400,
  Unauthorized 401, Missing 404, StaleReview/Conflict 409, Busy 503, Internal 500.
  Keep raw native diagnostics in private host artifacts.
- [ ] Add the frontend package/lockfile with `build` and `test` scripts, plus
  **`frontend/src/App.tsx`**, `api.ts`, `RunSteps.tsx`, `Result.tsx`. First Review
  discovers profiles; a profile-only project returns its available profiles with no prepared ID so the user can select one. Changing the directory or profile checkboxes clears
  its prepared ID and requires Review again. Start stores its request key until
  accepted. During Starting/Run/Stopping poll `/api/current` every 1 s with one request in flight; refresh restores the current
  run. Render emitted states and terminal durations, plus incomplete/truncated
  progress indicators. Stop remains available during Starting and Running.
- [ ] Project `ComposeRunReport` into execution error category, `writers_stopped`,
  `recording_finalized`, `boundary_clean` and `event_stream_healthy`. Persist a
  sanitized `console-report.json`; keep the original `report.json` private.
  Render these outcomes separately. A runner error without a report shows Unknown
  for unavailable outcomes and preserves the admission block.
- [ ] Register only `cpu.mcap` and `console-report.json` from the current run.
  Reuse [evidence::root/open](../../libs/workload-docker/src/evidence.rs) in the addon
  through a new `openArtifact(runId, kind)` binding, opening beneath the trusted
  artifact root without following symlinks; transfer
  ownership of the opened fd to the Node stream and close it on finish/disconnect.
  A download cannot name a host path. Missing/incomplete CPU evidence disables that
  download; replacing the current run leaves prior files on disk without indexing.

| Route | Contract |
|---|---|
| `POST /api/prepare` | `{directory, profiles}` → prepared ID, available profiles, review; no Engine launch |
| `POST /api/start` | `{preparedId, requestKey}` → 202/run ID; stale review → 409 |
| `GET /api/current` | Current lifecycle, bounded step snapshot and optional result; Idle if none |
| `POST /api/stop` | `{runId}` → accepted/already stopped; mismatched run → 409 |
| `GET /api/artifacts/:runId/:kind` | `cpu` or `report` only; authenticated streamed download |

**Validation**

Partial evidence (2026-10-29, working tree based on `3418acc`): `npm test` in `console/server` passed seven tests including route rejection, chunked body limits, descriptor traversal/symlink checks, 8 MiB streaming and disconnect closure. `npm test` in `console/frontend` passed six model and markup tests. Subsequent browser and packaged HTTP evidence is recorded below.

Chromium 1248 rendered the project, review, run, stopping, result and blocked fixtures. The review button reached the review screen, and project, run, stopping, result and blocked had no horizontal page overflow at 390 px. Representative captures are in `console/docs/screenshots/`.

Further browser evidence: project, starting, run, stopping, result, blocked, interrupted, unavailable-recording and progress-loss fixtures fit at both 1200 px and 390 px. Keyboard Tab, typing and Enter reached Review at both widths. The later checks below cover contrast and enabled-action focus order.

After correcting profile discovery, Chromium confirmed a declared profile can be discovered, selected and reviewed at both widths. The Node 22 frontend tests and production build passed again.

At 390 px, keyboard focus reached the enabled run, result and evidence actions in document order with visible outlines; stopping and interrupted states had no enabled actions. Calculated contrast for the shared body, muted, primary, status and notice text pairs ranged from 5.02:1 to 12.11:1.

- [ ] **`server/tests/http.test.ts`**: exact route/status matrix, oversized request,
  wrong credentials, cross-origin mutation and unknown fields. Native-call spies
  prove rejected requests never invoke Start/Stop.
- [ ] **`frontend/src/model.test.ts`**: change input invalidates review; lost Start
  response retries the same key; stale polling response cannot replace a newer
  run; step failure and cleanup failure render distinct outcomes.
- [ ] **`server/tests/artifacts.test.ts`**: traversal, wrong run ID and symlink swap
  fail; disconnect closes the fd. Stream an artifact larger than the stream buffer
  and verify backpressure without reading the entire file into memory.
- [x] Capture each concept state at 1200 px and 390 px widths. Verify shared
  action placement and state labels, keyboard/focus order, contrast, wrapping and
  no horizontal page overflow. Include Stopping, interrupted/Unknown, unavailable
  recording, progress loss and saved-recording/blocked-cleanup variants.
- [x] Run `npm test` and `npm run build` in both Console packages. Browser-check
  Review, Starting, Run, Stopping, Finished and Blocked with deterministic native
  fixtures before the real-Docker test in M5.

## M5 — Package and qualify the complete slice

**Work items**

- [ ] Add **`console/scripts/package.mts`** to build the addon, server and frontend
  into one directory, with `console.pbtxt.example` and launch instructions.
  First package: Linux x86_64 glibc, Node 22.12 or later; record the tested Compose version,
  native prerequisites and CPU-monitor image ID. No image build/pull in Console.
- [ ] Manually qualify the packaged server with a small Compose project and local
  images; record run IDs, outcomes and downloaded evidence beside this milestone.
  Keep focused component and integration tests for individual contracts.

Testing policy update (2026-10-29): the user requested removal of the branch's
standalone end-to-end harnesses. Removed packaged acceptance, package smoke and
simulated UI server scripts, their Compose fixtures and the acceptance-only MCAP
verifier. Historical evidence below records checks performed before removal.
After removal, all nine frontend tests, the frontend production build and
`git diff --check` passed. Application source is unchanged by this cleanup.

**Validation**

Partial evidence (2026-10-29, working tree based on `3418acc`): an initial Node 24 package passed startup, authenticated status and frontend delivery against a local API stub. The first real-Docker acceptance attempt could not start because the unprivileged server could not connect to `/var/run/docker.sock`; the user authorized sudo for the subsequent acceptance run. Docker Compose was v5.6.0.

Further evidence: package `/tmp/doramcap-console-package-1029c` passed the startup smoke test on Node 22.23.3. The separate real-Docker harness, run with authorized sudo access, passed two captures, stale `.env`, missing image, early exit and interrupted-server restart. Run `1093fcff1982a8529a65023d1594d16c` produced session `1793319633912283758-1` with 43 snapshots; run `c5c2a54eb9555c1a4a08e6be322bc3c0` produced session `1793319640787362859-1` with 41. Both MCAP footers and contiguous sequences verified. The first harness attempt incorrectly expected every early exit to block even when cleanup succeeded; the assertion was corrected and the full harness rerun. Final-revision and remaining acceptance checks remain open.

Final package evidence: Node 22.23.3 package startup passed. The separate real-Docker harness passed again: run `13a88f883c3adaafa0375c5f008e0e86` produced session `1793320276564780882-1` with 55 snapshots; run `4a1ca61ad09993a03286f52fdfd6fb49` produced session `1793320287005665264-1` with 50. Both had valid footers and contiguous sequences. Local default/all-feature workspace tests, default/all-feature Clippy, no-default storage tests, boundaries, formatting and snapshot synchronization passed. Tests explicitly ignored by the workspace suite, including the high-memlock case, were not run as part of those commands. The clean standalone all-feature examples/binaries build passed. ARM64 jobs 495–499 succeeded; job 499 includes the final implementation and expanded tests. Both local workspace configurations and both Clippy configurations passed again on the final implementation. The final clean all-feature examples/binaries build passed.

The expanded harness passed after the final clean build: runs `e35d7e54ae6aa7c84286ec96dcbea764` and `bf28afa230a589562a60c19aedbaad8c` produced distinct sessions with 42 snapshots each and valid footers. A separate active run survived graceful server shutdown as a finalized, clean result restored after restart. Stale input, missing image, early exit and killed-server restart checks also passed.

ARM64 worker job 500 passed on the completed implementation and expanded acceptance source.

- [x] Successful run: review default/profile selections, Start, observe real
  PrepareRun/LaunchProjects/ObserveProjects events, Stop, observe finalization and
  removal, download MCAP/report. Reuse the existing footer/sequence checks from
  `native_recorded_workflow`; verify fresh capture session IDs across two runs.
- [x] Failure runs: missing image fails Review; changed effective config rejects
  Start; early exit reports execution failure; failed cleanup blocks next Start.
  Kill the server during capture and assert restart is Blocked without launching
  new resources. Explicitly clean up the harness's owned resources afterward.
- [ ] Open the packaged server from a second machine and complete the same flow;
  reload during capture and after a clean restart. Verify source Compose files
  unchanged and the latest result restored. No history selector is required.
  Second-machine browser acceptance was explicitly skipped at the user's request.
- [x] Run all local [CI checks](../../.github/workflows/ci.yml) on final source using
  `--profile ci --locked`; report default/all-feature Clippy and workspace tests,
  no-default storage tests, builds, boundaries, formatting and snapshot tests separately.
- [x] Run new Docker/native test executables built unprivileged; elevate only the
  executable when capabilities are required, following repository credential rules.
  Submit final source to the serial ARM64 worker and verify its result.
  Manual packaged Docker qualification is separate from the worker suite. The
  standalone Console harness was subsequently removed at the user's request.

Acceptance evidence is recorded beside each milestone above; unchecked items remain incomplete.

## M6 — Workflow stages and live activity timeline

The requested timeline uses existing workflow telemetry, following the activity demo's moving SVG view. It does not add event replay or a new telemetry source.

**Work items**

- [x] Show top-level Compose lifecycle steps with status and elapsed duration; omit periodic checks from this list.
- [x] Show a moving, bounded timeline of workflow events, including health checks, with pause and time-window controls.
- [x] Retain lifecycle rows within the existing bounded reducer as old periodic events are evicted; preserve loss accounting and full invocation IDs.

**Validation**

Evidence (2026-10-29): the nine frontend tests and production build passed, and five native reducer tests passed, including 2,000 observations with lifecycle retention and exact eviction accounting. Chromium checks at 1200 px and 390 px verified the stage list excludes periodic checks, the timeline moves and pauses, the running duration advances, and telemetry loss remains visible.

The earlier interactive demo run is blocked with cleanup unverified; its recording was saved. Manual real-Docker qualification for this change is pending resolution of that run, so no second run is started behind the block. ARM64 preliminary job 502 passed; final-source job 505 passed on `a314bea`.

The updated package passed its startup smoke test and was installed for the interactive demo. Restart restored the same blocked run with its CPU recording and report available.

Local default-feature workspace tests, storage without default features, both Clippy variants, formatting, boundaries, snapshot synchronization and the clean all-feature examples/binaries build passed. The first all-feature workspace attempt ran out of disk space while linking an example; the lease's disposable build artifacts were removed and the clean retry passed. Server tests and the packaged startup smoke test also passed. Workspace-ignored capability and CUDA tests were not run by those commands.

- [x] Test stage filtering, 64-bit timestamp arithmetic, moving/paused timeline, loss display and bounded lifecycle retention.
- [x] Check the run view in Chromium at desktop and narrow widths; run frontend tests/build and Rust reducer tests.
- [ ] Run applicable local CI, final-source ARM64 validation and manual packaged real-Docker qualification; record failures and skipped checks. The automated Console acceptance harness was removed at the user's request.

Manual recovery and qualification (2026-10-29): the blocked demo report showed
forced/unclean workload shutdown diagnostics, verified Docker removal and completed
filesystem finalization. Host checks confirmed no run/project containers, networks,
volumes or overlay mounts; the marker was archived beside its evidence before
restart. The demo services now handle SIGTERM. Manual run
`b28ac30eb9bc11aa0937e25c8e714027` with profile `extra` reached CPU readiness,
served Hello World and stopped with Completed execution, finalized recording and
verified cleanup; both downloads are available. The blocked screen now explains
manual recovery, documented in [recovery.md](recovery.md). All nine frontend tests,
the production build and diff whitespace checks passed. Full CI was not rerun for
this guidance-only change; earlier CI evidence remains tied to its recorded revision.

## M7 — Interactive run inspection and resource panels

The user requested clickable event details, MCAP rate/size, CPU totals across
cores, and navigation from Result back to the current run's retained activity.
This adds bounded observation and a completed snapshot; it provides no event
replay or run-history selector.

**Work items**

- [x] Select timeline events with mouse or keyboard, inspect timing/identity,
  parent/dependencies and redacted failure status; browse elapsed time.
- [x] Navigate between 03 Run and 04 Result after completion, with frozen timing
  and no execution controls in the completed view.
- [x] Add protobuf `RunMetric` and additive `/api/current` fields for up to 120
  samples, MCAP bytes/file-growth rate and CPU utilization. Existing workflow
  events already supply detail fields; these metric fields are new Console APIs.
- [x] Read the existing CPU protobuf sidecar with bounded header traversal and
  at most two 8 MiB frames per tick, using the existing telemetry-protocol crate.
  Sample on blocking workers away from the execution owner and JavaScript loop.
- [x] Sum inclusive cgroups once across cores; exclude unavailable root and
  overlapping virtual kernel rows. Label recorded cgroup coverage and use 100%
  per fully occupied core. Counter resets/invalid coverage remain unavailable.
- [x] Drain telemetry and await final metric sampling/checkpoint before shutdown
  acknowledgement. Persist redacted bounded terminal activity in `activity.pb`;
  restore only the marker's matching terminal run after restart. Old runs without
  a snapshot explicitly lack retained events.

**Validation**

Initial evidence: 14 frontend component/model tests and the production build
passed. Native unit tests passed (14 passed, five Docker/capability tests ignored),
including exact 64-bit deltas without inclusive/virtual double-counting, partial
frames, counter resets, frame bounds, symlink rejection and matching-run restoration.

Final evidence (2026-10-29): package `/tmp/doramcap-console-panels-final` ran
`599a71ac143838ac9d560cc5bbe3ceb4` with profile `extra`. Live inspection showed
MCAP growth and nonzero CPU totals across 20 cores. Stop completed execution,
finalized the 38,788,677-byte recording and verified cleanup. After graceful
restart, all 258 invocation rows and 68 metric samples were byte-for-byte equal
in the decoded status arrays. Result navigation and the 03 rail opened the
read-only view, with no Stop control. Selected event details and both panels fit
at 1200 px and 390 px; screenshots are in `screenshots/run-inspector*.png` and
`screenshots/completed-activity*.png`.

Final-source local CI passed: default/all-feature Clippy and workspace tests,
no-default storage tests, formatting, boundaries, snapshot synchronization and
all-feature example/binary builds (`--profile ci --locked`). Fourteen frontend
tests and its production build, fourteen native unit tests and seven server
tests/build passed. An initial Clippy borrow-scope warning and a test fixture's
TypeScript type error were fixed before the final checks. Only this lease's
build artifacts were cleaned between configurations to stay within disk space.
Workspace-ignored capability/CUDA/high-memlock cases remain skipped; the packaged
Docker flow above was checked manually. ARM64 worker job 508 passed the final
implementation snapshot; jobs 506/507 covered earlier snapshots. No standalone
E2E harness was reintroduced.

- [x] Verify selected details, historical navigation, live recording/CPU samples,
  completed freeze and restoration against a real packaged Docker run.
- [x] Check narrow layout and retained failure/unavailable states using components.
- [x] Run local CI and final-source ARM64 checks; record exact results and skips.

## M8 — Binary 10 Hz telemetry and existing Rust counters

The user requested the demos' binary Protobuf delivery pattern, direct timeline
selection, and useful existing Rust metrics in place of explanatory panel copy.

**Work items**

- [x] Add authenticated `/api/live`: four-byte little-endian framing around
  Rust-encoded Status checkpoints/deltas at 10 Hz. Keep control routes compatible.
  Use opaque Rust cursors, bounded frames, four-client admission, slow-client
  backpressure/timeouts and fresh checkpoints after reconnect; no JSON live polling.
- [x] Reuse generated protobufjs codecs already used by the demos. Merge exact
  invocation identities, cap rows at 512 and metric samples at 1200; avoid copying
  unchanged history and resending unchanged CPU payloads on heartbeats.
- [x] Sample existing output at 10 Hz away from execution's owner thread. Reuse
  `doramcap.telemetry.v1.Snapshot` for the last two CPU frames; expose no dictionary
  paths. Reuse existing sequence, cgroup runtime, flags, parents, dropped-batch and
  capacity-overflow fields for CPU ranking and capture health. Transport cursor
  metadata is new; no producer measurement fields were invented.
- [x] Remove the retained-event picker and both explanatory paragraphs. Keep
  keyboard/direct timeline selection, larger hit targets, nearest-event selection
  within lanes, and parent/dependency navigation. Put nested checks in Details.
- [x] Leave io_uring rates absent: the deployed system-monitor publishes CPU
  snapshots, but its embedded recorder does not export storage writer counters.
  Preserve that distinction from storage-demo's process-local metrics.

**Validation**

Focused evidence: 17 frontend tests, 15 native unit tests (five ignored Docker
cases) and eight server tests passed. A real large-checkpoint run exposed a corked
write/drain stall missed by the original small-frame mock. The route now writes
one framed buffer; the regression test uses a 200 KB message and passes.
A real completed-run connection then delivered 21 frames in 1.99 seconds (10.06 Hz),
with a 253,047-byte checkpoint and 118-byte median heartbeats. Direct timeline
clicks opened details, the picker was absent, and browser requests used only
`/api/live`. The 390 px document width remained 390 px.

Final packaged evidence (2026-10-29): run
`c0b3896e618477dbf8c12fd7f950d77c` reached monitor readiness and delivered 21
active frames over 1.992 seconds (10.04 Hz), including 19 new metric samples and
updated producer CPU snapshots. Stop completed execution, finalized recording and
verified cleanup. Restart restored 450 metric samples, 255 invocation rows and
two CPU snapshots; metrics coverage remained complete. A real server restart
showed the disconnect indicator and automatically reconnected while retaining
03 Run. The terminal WriteRunReport marker at the plot edge was directly
clickable; the event picker and both removed paragraphs are absent. Final
completed screenshots are `screenshots/completed-activity*.png`.

Run `47f3f72d071b6d497361d3276cb5f360` hit the Compose system-project launch
60-second deadline before producing monitor data. It reported failed execution,
unavailable recording and verified cleanup; the successful run above followed
that clean boundary. The earlier live-browser attempt also exposed an edge hit
target clipped outside the plot; short markers now remain within plot bounds.

Final checks passed: 17 frontend tests and production build, 15 native unit tests
(five Docker/capability cases ignored), eight server tests/build, both Clippy and
workspace-test configurations, storage without default features, all-feature
examples/binaries, formatting, boundaries and snapshot synchronization. All Cargo
checks used the CI profile and locked dependencies, built unprivileged. ARM64 job
512 initially failed the same nested-if Clippy warning found locally; it was
corrected. Jobs 513 and 514 passed; job 514 includes the final event hit-target
fix. Workspace-ignored capability/CUDA/high-memlock cases remain skipped. No E2E
harness was added.

- [x] Verify live sampling, reconnect/restoration and final screenshots against
  the packaged Docker app; record actual outcomes and unavailable cases.
- [x] Run final-source local CI, frontend/server builds and ARM64 validation;
  record results and skips separately.

## M9 — Step rows and explicit event flows

**Work items**

- [x] Give each full Rust step type a separate timeline row. Repeated invocations
  share that row; labels expose the full type name on hover. Keep rows present
  while browsing windows and preserve direct/keyboard event inspection.
- [x] Draw reported dependencies from producer completion to consumer start and
  parent links from parent start to child start. Resolve exact invocation IDs
  within their session/executor; omit links whose endpoints were evicted.
  Render arrowheads, curved same-row links, clipped cross-window links and
  selection highlighting. Reuse existing telemetry; no new APIs or dependencies.

**Validation**

Frontend: 20 tests and the production build passed. Focused cases cover repeated
names, full-name separation, precise scoped IDs, missing endpoints, duplicate
references and same-row/cross-window arrows. Headless Chromium against the
packaged completed Docker run displayed 20 rows and 218 InspectCompose markers
in the 60-second window. Selecting invocation 340 opened its precise details
and highlighted its one reported parent link from invocation 338. At 390 px,
the document remained 390 px wide; the timeline scrolls within its panel.
Updated `screenshots/completed-activity*.png` capture this real retained run.

- [x] Verify grouping, relationships, event selection and narrow-screen layout.
Keyboard Enter opened the selected InspectCompose event; following parent 338
opened ObserveProjects and highlighted its reported relationships.

Local CI passed on the final application source: default/all-feature Clippy and
workspace tests, storage without default features, all-feature examples/binaries,
formatting, library boundaries and snapshot synchronization. Builds used the CI
profile and locked dependencies, unprivileged. Logs: `/tmp/console-rows-*.log`.
ARM64 job 515 passed the same application source, including native accounting and
allocation checks. Existing ignored capability/CUDA/high-memlock tests remain
skipped. This rendering change reused the completed real Docker run above;
no new execution acceptance or second-machine browser run was performed.

- [x] Run local CI and ARM64 on the final source; record results separately.

## M10 — Concise interface copy

**Work items**

- [x] Remove routine retention limits, timeline subtitles, selection hints,
  payload explanations and redundant metric/Stop/download commentary. Keep
  statuses, controls, units, empty states and actionable failure/recovery messages.
- [x] Simplify missing-data wording and remove the redundant successful-result
  readiness panel. Leave actual incomplete-progress and cleanup warnings visible.

**Validation**

All 20 frontend component/model tests and the production build passed. Updated
assertions cover the absent retention/payload prose and concise empty states.
Headless Chromium verified the removed copy was absent on the packaged completed
run, event selection still opened details, and the 390 px page remained 390 px
wide. Refreshed `screenshots/completed-activity*.png` show the simplified view.
No new Docker execution was needed for this copy/layout change.

- [x] Verify the rendered copy, event selection and narrow layout.
Final-source local CI passed: both Clippy/workspace configurations, storage
without default features, examples/binaries, formatting, boundaries and snapshot
synchronization (CI profile, locked dependencies, unprivileged builds). ARM64
job 516 passed. Logs: `/tmp/console-copy-*.log`. Existing ignored capability,
CUDA and high-memlock cases remain skipped; second-machine browser acceptance
remains skipped by user instruction.

- [x] Record final-source local CI and ARM64 results.

## M11 — Remove generic project warning

**Work items**

- [x] Remove the blanket policy/configuration banner from successful reviews.
  Compose diagnostics stay private, including routine process-only readiness;
  actionable prompts such as selecting a profile remain public. Raw diagnostics
  can include interpolated secrets and are never forwarded to the browser.

**Validation**

The real Docker review test passed with added assertions: a project without
healthchecks still has private process-level readiness diagnostics, while its
public review has no generic warning. Builds ran unprivileged; only the test
executable was elevated. All 15 native unit tests, six project/state integration tests and eight server
tests passed. The default native command ignored ten Docker/capability tests;
one Docker review test was explicitly run and passed above. The packaged native module
was replaced and the server gracefully restarted after confirming FINISHED.
ARM64 job 517 passed the final application source.

- [x] Verify diagnostics remain private and the successful review has no banner.
Full local CI passed on the final application source: both Clippy/workspace
configurations, storage without default features, examples/binaries, formatting,
boundaries and snapshot synchronization (CI profile and locked dependencies).
Logs: `/tmp/console-warning-*.log`. The restarted app restored FINISHED run
`41043c3092c58bd4a8f8b2f429ef2c54`. Existing ignored workspace capability/CUDA/
high-memlock tests and second-machine browser acceptance remain skipped.

- [x] Record full local CI and confirm packaged restart.

## M12 — Review-time system app selection

This extends the original read-only CPU-recording concept with a System apps
checkbox panel on Review. The existing injectable template is CPU recording;
Dora flow injection remains outside Console's supported configuration.

**Work items**

- [x] Add optional `SystemAppSelection` to PrepareRequest and public Review app
  options. Omission uses host defaults; an explicit empty list selects no apps.
  Reject unknown, duplicate and unconfigured IDs. Host configuration supplies
  trusted images/templates; browser requests carry IDs only.
- [x] Reprepare after checkbox changes and disable Start until the new review
  succeeds. Bind Start to that review's selected generated SystemAppsConfig,
  pin selected monitor images and revalidate their presence/absence before launch.
- [x] Persist selection in Marker/Status and the public report. With CPU disabled,
  skip the monitor, CPU sampling/readiness and MCAP validation; show Disabled
  separately from recording failure, with workflow activity/report still usable.
  Existing snapshots with no selection field retain their historical CPU meaning.
- [x] Allow host configuration without a CPU monitor. Keep protobuf configuration
  and existing templates/dependencies. No new crate or producer fields.

**Validation**

21 frontend tests/build and eight server tests passed. New focused cases cover
checkbox state, disabled controls, unavailable-vs-disabled outcomes, optional HTTP
selection and explicit empty lists. Rust selection tests cover defaults, empty,
unknown/duplicate IDs and unavailable apps. The Docker review test also verifies
that disabling CPU bypasses resolution of a deliberately missing monitor image.

Real Docker runs passed with CPU enabled (`c72b05f17866e70efd681040a8529683`,
52 snapshots) and disabled (`b6323ce99d15b18ff1804488d41f73d9`). Both exercised
Start retries, Stop and verified cleanup. The disabled run had no MCAP/CPU
samples, a null recording outcome and a persisted empty selection.
The initial disabled fixture stopped before its Node shutdown handler was ready;
cleanup correctly reported unclean shutdown. Adding a healthcheck to the fixture
made the successful rerun exercise a ready workload; no recovery bypass was added.

- [x] Verify real Docker execution with selected and unselected CPU recording.
Packaged Chromium verification used an isolated review-only data root with the
same built server/native/frontend. The default CPU checkbox was checked; an
injected failed reprepare left Start disabled, Retry review created a new review
with CPU unchecked, and checking CPU again produced another reviewed selection.
The 390 px document remained 390 px wide. Updated `screenshots/review.png` and
`review-narrow.png` show the real resolved project. The temporary validation
server was stopped; the interactive app remains hosted at its existing URL.
Initial packaging checks caught TypeScript nullability/incomplete test-fixture
errors; both were corrected before the successful tests and production builds.

- [x] Verify the packaged checkbox/review interaction, failure retry and screenshots.
Final source passed 21 frontend tests/build, eight server tests/build, both
Clippy/workspace configurations, storage without default features, examples/
binaries, formatting, boundaries and snapshot synchronization. Cargo used the
CI profile and locked dependencies with unprivileged builds. ARM64 jobs 518 and
519 passed; 519 includes the final TypeScript test-fixture corrections. Three
ignored Docker tests were explicitly run above (selected run, unselected run and
review); the other eight native Docker/capability cases and existing workspace
capability/CUDA/high-memlock skips remain skipped. Second-machine browser
acceptance remains skipped. Logs: `/tmp/console-apps-*.log`.

- [x] Record final-source local CI and ARM64 results.

## M13 — Named CPU consumers

**Work items**

- [x] Display actual cgroup names from the producer's existing dictionary in
  Top CPU consumers. Keep exact numeric IDs in tooltips and as missing-name
  fallback. Task names are not present in the current producer protocol.
- [x] Add a public display-name projection (up to 512 names, 128 characters each),
  using the final cgroup path component. Keep ancestor host paths private.
  Send names with CPU frames, retain them across heartbeats and checkpoint them.
- [x] Resolve names for older completed checkpoints from a matching final
  `cpu.pb` dictionary. Bound reads and require matching snapshot sequence;
  unavailable/mismatched dictionaries keep the numeric fallback.

**Validation**

23 frontend tests/build, 17 native unit tests, six project/state integration
checks and eight server tests/build passed. Focused tests cover exact IDs above
2^53, heartbeat retention, fresh-frame reset, named/numeric rendering, private
ancestor exclusion, length limits and legacy checkpoint backfill. An initial
TypeScript fixture used string IDs directly in generated Long fields; converting
through the generated protobuf factory corrected the build before validation.

After the user's active run finished, the packaged app restarted and restored
151 cgroup names for run `cdc8376960dae20c815fd5a84fb7f2dc`. Chromium displayed
actual names (including `session-7.scope` and
`gvfs-udisks2-volume-monitor.service`), event inspection still worked, and the
390 px page remained 390 px wide. Updated completed-activity screenshots show
this real recorded run. No new Docker execution was needed for this label change.

- [x] Verify names, privacy bounds, exact identity and completed-run restoration.
Full local CI passed on the final application source: both Clippy/workspace
configurations, storage without default features, examples/binaries, formatting,
boundaries and snapshot synchronization (CI profile, locked dependencies,
unprivileged builds). ARM64 job 520 passed. The eleven ignored native Docker/
capability cases were not rerun for this display change; prior M12 execution
evidence remains above. Existing workspace capability/CUDA/high-memlock and
second-machine browser skips remain unchanged. Logs: `/tmp/console-names-*.log`.

- [x] Record final-source local CI and ARM64 results.

## M14 — Timeline row chronology

**Work items**

- [x] Order step-name rows by their earliest recorded invocation start, preserving
  one row per full step type. Incoming array order and delayed/restored startup
  rows must not place launch stages below cleanup. Keep timestamps and explicit
  flow relationships unchanged.

**Validation**

24 frontend tests and the production build passed. The added model case supplies
report/cleanup events before startup events and verifies chronological row order
with repeated invocations still grouped. Chromium verified launch above shutdown
and shutdown above report against the packaged completed run; the 390 px page
remained 390 px wide. Completed-activity screenshots were refreshed.

The current run's private Docker events recorded three creates/starts and three
kills/stops/deaths/removals, with zero starts after the first shutdown event.
Rust invokes launch groups before its observation loop, then quiesces groups in
reverse order, finalizes evidence/filesystems, removes resources and writes the
report. `LaunchDoraFlows` is an empty generic step for this Compose-only run.

- [x] Verify chronological grouping, narrow layout and actual container ordering.
Full local CI passed on the final application source: both Clippy/workspace
configurations, storage without default features, examples/binaries, formatting,
boundaries and snapshot synchronization (CI profile, locked dependencies,
unprivileged builds). ARM64 job 521 passed. Existing native/workspace ignored
capability, CUDA and high-memlock tests and second-machine browser skips remain
unchanged. Logs: `/tmp/console-order-*.log`.

- [x] Record final-source local CI and ARM64 results.

## M15 — Expanding activity card

**Work items**

- [x] Remove the timeline's 600 px height limit and internal vertical scrolling.
  Let the activity card grow with its rows; retain horizontal scrolling on
  narrow screens.

**Validation**

24 frontend tests and the production build passed. Chromium measured matching
container/content heights at 1200 px (482/482 px) and 390 px (664/664 px), with
no maximum height or internal vertical overflow. Document width matched each
viewport. The hosted static assets and completed-activity screenshots are updated.

- [x] Verify card expansion and narrow-screen containment in the packaged app.
Full local CI passed on the final application source: both Clippy/workspace
configurations, storage without default features, examples/binaries, formatting,
boundaries and snapshot synchronization (CI profile, locked dependencies,
unprivileged builds). ARM64 job 522 passed. Existing ignored capability/CUDA/
high-memlock cases and the second-machine browser skip remain unchanged. Logs:
`/tmp/console-expand-*.log`.

- [x] Record final-source local CI and ARM64 results.
