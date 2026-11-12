# Run directory layers: implementation plan

Created November 11, 2026. Updated November 11, 2026: reworked around overlay layers.
Status: milestone 1 implemented in review ([#318](https://github.com/Cutuy/doramcap/pull/318)),
milestone 3 implemented in review ([#320](https://github.com/Cutuy/doramcap/pull/320)), the rest planned.
[Architecture](111026-file-change-report.md).

## Order

| Milestone | Starts | Reason |
| --- | --- | --- |
| 1: extract capture | Done, in review | Moves overlays and change records into `host-filesystem`; behavior unchanged. |
| 3: report in Console | Done, in review, after 1 | Reads change records, whatever produced them. |
| 2: change list from upper layers | After 1 | Replaces baseline scans; every later milestone builds on it. |
| 4: systemd overlays | After 2 | systemd starts on the final change-list mechanism. |
| 5: keep with the run | After 2 and 3 | Keeps upper layers and uses milestone 3's download route. |

Milestone numbers 1 and 3 keep their earlier meaning, so PR references stay valid.

The [admission lock](https://github.com/Cutuy/doramcap/pull/315) and the
[shared apps crate](https://github.com/Cutuy/doramcap/pull/317) also move code
out of `workload-docker` and proceed in parallel. The apps crate's last milestone
waits for milestone 1 here, since both reshape the Compose run sequence. The
[Holoscan overview](https://github.com/Cutuy/doramcap/pull/264) holds the order
across all of this work.

## Milestone 1: Extract Linux capture and preserve Docker behavior

**Work items**

- [ ] Move the existing overlay trees, mapping, direct upper-layer actions, metadata
  traversal, record readers/writers and schema into `host-filesystem`. Keep this crate
  independent of Docker, workloads and `orchestrator`.
- [ ] Put concrete shared workflow steps in `workload-lifecycle`. Extract the common
  capture/finalization sequence from Compose and use it without a backend plugin framework.
- [ ] Keep Docker socket namespace verification, Engine layer diffs, volume discovery
  and owned-container checks in `docker-engine`. Supply their observations to the common report.
- [ ] Replace Docker-specific mount input types at the shared boundary with generated
  source/consumer/access records. Keep propagation and SELinux validation in the Docker adapter.
- [ ] Preserve `doramcap.filesystem.v1` record tags, path bytes, existing category values,
  view/checkpoint layouts, reset behavior, write-back/discard and Compose step lineage.
  Update owners, public names, callers, API doc comments and boundary rules together; add no forwarding aliases.
- [ ] Reuse the repository's existing dependencies and implementations. No new external
  crate or custom capture algorithm is selected by this plan.

**Validation**

- [ ] Existing real OverlayFS cases pass after the move: isolation, shared aliases,
  additions, modifications, deletion/opaque directories, write-back and discard.
- [ ] Preserve busy-mount, full-disk, external-lower-change and interrupted-apply behavior.
- [ ] Existing Compose file-change manifests and independent readback remain compatible.
- [ ] Boundary checks, focused release tests, formatting and strict Clippy pass with locked dependencies.

## Milestone 2: Read the change list from upper layers

**Work items**

- [ ] In `host-filesystem`, walk each stopped upper layer into change records: added
  when absent from the lower layer, modified when present, deleted for whiteouts and
  opaque directories. Take "before" metadata from the lower layer, "after" from the upper.
  Reuse the existing upper-layer interpretation that write back already uses.
- [ ] Keep the `doramcap.filesystem.v1` change records, categories, manifests and report
  checkpoint unchanged, so milestone 3 needs no change.
- [ ] Mount each Compose run output folder as an overlay with an empty lower layer.
- [ ] Apply the named-volume decision: overlay local volumes like binds, or list them as
  not tracked.
- [ ] Apply the copied-up-file decision.
- [ ] Delete baseline scans, their records and the metadata comparison once nothing reads them.
  Keep Docker's container-layer diff.

**Validation**

- [ ] Real OverlayFS fixtures: additions, modifications, deletions, opaque directories,
  renames, symlinks, non-UTF-8 names and metadata-only changes produce the expected records.
- [ ] The same Compose writes produce the same records as the scan-based list did, apart
  from the copied-up-file decision.
- [ ] Listing cost follows the size of the upper layer: a large unchanged source lists quickly.
- [ ] Write back, discard, busy mounts, full disk and interrupted runs behave as before.
- [ ] Boundary checks, focused release tests, formatting and strict Clippy pass with locked dependencies.

## Milestone 3: Persist one report and expose preview plus JSON Lines

**Work items**

- [ ] Use shared generated types for category counts, source coverage, report state
  and run-relative manifest references. Preserve absence separately from zero changes.
- [ ] Carry the result through `ExecutionResult` and the Console run result for both
  backends. Keep full lists and unbounded source notes out of progress events and live status.
- [ ] Validate manifest references beneath the private run root using existing safe
  file opening. Save the immutable report checkpoint before destructive finalization;
  sync manifest files and their directory, atomically publish the completion checkpoint,
  and save its Console reference with the existing marker/History transaction. A crash
  before checkpoint publication cannot establish a completed report.
- [ ] Add file-change summary and manifest/artifact references to `console-report.json`.
  Read the saved report for current and past results; do not reconstruct it from progress.
- [ ] Derive a preview within the existing encoded status/detail budgets. Include the
  shown/total count and incomplete-observation state. Choose a named display budget
  using real rendered fixtures; do not impose a capture limit or silently cut individual records.
- [ ] Add a file-changes artifact kind to the existing authenticated download route.
  Stream JSON Lines from saved manifests with generated protobuf codecs, backpressure
  and descriptor cleanup on disconnect. No second persistent JSON copy or browsing API.
- [ ] Export category summaries and full coverage notes alongside every collected
  change, plus an end record confirming completed traversal. Distinguish export
  completion from capture coverage; abort on malformed/truncated records or count mismatch.
- [ ] Export 64-bit values as decimal strings and raw path/symlink bytes as base64 with
  readable display fields. Render paths as text; never treat them as content-open links.
- [ ] Show a File changes section with counts, coverage, preview and Download file changes.
  Reuse it for History details. Availability is independent of recording finalization.
- [ ] Keep recording-byte comparisons limited to declared recordings when adding a new
  artifact kind. Preserve old-run decoding, artifact authorization and existing filenames.

**Validation**

- [ ] Native result, protobuf, saved report, History detail and JSON report retain the same counts and coverage.
- [ ] Large fixtures exceed the preview budget; an independent JSON Lines reader recovers
  every change and coverage note, verifies totals and sees the completion record.
- [ ] Missing/truncated manifests, oversized preview records, invalid UTF-8 names,
  absent metadata, complete zero changes and failed capture render distinctly.
- [ ] Test cross-run references, traversal, symlink escapes, session authorization,
  slow downloads and disconnects. A failed export cannot look complete.
- [ ] Real Compose and systemd runs retain the preview and download after refresh,
  navigation and Console restart, including failed runs with usable file reports.
- [ ] Check the result on desktop and narrow screens; preserve recording downloads and byte totals.

## Milestone 4: Give systemd services the same overlays

**Work items**

- [ ] Read each unit's declared writable binds: from the app's settings for services
  Console creates, and from the unit file and its drop-ins through the service manager
  for your own unit files. Reuse the existing input copies; add no unit-file parser.
- [ ] Mount one overlay per writable bind before any unit starts, through the shared
  lifecycle. Point each bind at its merged view with an owned property or drop-in, keeping
  its app path and access.
- [ ] Delete the automatic same-path bind of a created service's working directory.
- [ ] Add a test that the Dora coordinator and daemon keep `/runtime` under their
  declared output bind.
- [ ] Review lists your own unit files as partly tracked: only their declared binds are.
- [ ] Stop all attempted units and release their hold on the views before unmounting.
  Then reuse milestone 2's listing and the shared write policy.
- [ ] Extend the systemd run report with the shared file-change result. Keep recording
  checks, host `/dev/shm` observation and machine snapshots unchanged.
- [ ] Carry ownership records into interrupted-run reset: stop units, then discard leftover views.

**Validation**

- [ ] Real created services and unit files see their views at their app paths; writes
  stay isolated during the run and reach the source after write back.
- [ ] Equivalent Compose and systemd fixtures produce equivalent change records.
- [ ] A created service whose working directory has no mount under it fails with
  systemd's own error; Dora's system apps start unchanged.
- [ ] Preparation rollback, cancellation before and during startup, busy views, report-save
  failure and Console restart.
- [ ] Build unprivileged; elevate only capability-required test executables. All owned
  mounts and units are gone after success and kept as specified after failure.

## Milestone 5: Keep a run's files with the run

**Work items**

- [ ] Add keep with the run beside write back and discard in `host-filesystem`: copy each
  upper layer's files into the run folder and record its deletions. Leave the source untouched.
- [ ] Remove the view's run layer only after the copy is verified. A failed copy keeps the
  upper data, as a failed write back does.
- [ ] Record the policy and the kept location in the file-change report.
- [ ] Add one Console setting selecting write back or keep with the run, default write back.
  Pass it to Compose and systemd runs.
- [ ] Show the policy at Review and the kept location with the result. Add no file browser.
- [ ] Add a download action for kept files and log dumps as one new kind on the
  authenticated artifact route. The request names a saved change-list entry or a program,
  never a path. The server opens the file only inside the run's kept-files or log folder,
  by its raw name bytes. Send every file as an attachment with an encoded filename.

**Validation**

- [ ] The kept files match the report's added and modified entries exactly.
- [ ] The source folder is byte-identical before and after a keep run.
- [ ] Deletions appear in the report and remove nothing.
- [ ] A second run starts from the original source and keeps its own separate files.
- [ ] Full disk and an interrupted copy keep the upper data and block verified cleanup.
- [ ] The run layer and the run folder on different filesystems.
- [ ] A kept file, one with a non-UTF-8 name and a log dump download intact. Requests for
  Console's run records, another run's entries, traversal and symlink escapes are refused.
  A kept HTML file downloads and never renders in Console.
- [ ] Projects with no setting behave as before.

## Integration and qualification

**Work items**

- [ ] Publish focused implementation PRs in the order above.
- [ ] Record completed and deferred checks here as implementation lands.

**Validation**

- [ ] Run full local CI on the final implementation source revision at integration:
  boundaries, formatting, default/all-feature Clippy and workspace tests, storage without
  default features and snapshot synchronization. Use release/CI profiles and locked dependencies.
- [ ] Run ARM64 worker validation and report it separately from local results.
- [ ] Qualify systemd view visibility, OverlayFS backing and capabilities on a real
  packaged target. Unit tests alone do not establish this deployment contract.
- [ ] Keep Holoscan, CUDA/GPU/IPC and sensor qualification explicit separate work.

Design validation: source/guide review, local links, SVG XML/rendering and whitespace checks pass.
Runtime checks above are deferred until implementation; this design PR changes documentation only.
