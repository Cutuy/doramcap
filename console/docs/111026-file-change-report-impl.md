# File changes in Console: implementation plan

Created November 11, 2026. Status: planned; application code unchanged.
[Architecture](111026-file-change-report.md).

## Order

Milestones are numbered by topic. Two can start at once and two wait:

| Milestone | Starts | Reason |
| --- | --- | --- |
| 1: extract capture | Now | Everything that reuses capture builds on the moved code. |
| 3: report in Console | Now, alongside milestone 1 | Capture already works for Compose. It builds on report types that milestone 1 relocates, which is an ordinary code conflict. |
| 4: keep with the run | After milestones 1 and 3 | It changes the finalization that milestone 1 relocates and uses milestone 3's download route. |
| 2: systemd capture | After milestone 1 | systemd cannot use the capture code until it leaves the Docker crate. Until it lands, a systemd run reads as unavailable. |

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

## Milestone 2: Wire systemd into the same capture lifecycle

**Work items**

- [ ] Resolve typed binds and effective native-unit bind properties using the service
  manager. Reuse captured unit/drop-in inputs; do not build another unit-file parser.
  Record sources, targets, consumers, optional-source behavior, access flags and exclusions.
- [ ] Resolve each unit's explicitly declared working directory through the service
  manager and capture it as a source. Skip an undeclared one, the filesystem root and a
  home directory shorthand, and list each with its reason. List declared state, log and
  cache directories as declared and uncaptured.
- [ ] Review shows known capture sources and unresolved native sources explicitly.
  At Start resolve the effective inventory before writers start; a changed launch
  configuration follows the existing stale-review check.
- [ ] Separate runtime/source preparation from starting writers where needed. The
  shared lifecycle owns the capture from the first attempted mount through verified cleanup.
- [ ] Create the same views as Docker. Rewrite supported bind sources through owned
  D-Bus properties/drop-ins and safe aliases, preserving targets, access and native settings.
- [ ] Verify that the systemd manager can resolve the mounted views. Keep the existing
  capability requirements; fail preparation when visibility is unqualified. Do not change
  persistent mount propagation or service-manager configuration as a workaround.
- [ ] Collect from declared application output roots through the same record writer.
  Preserve the current exclusion of controller-owned artifacts; report partial coverage
  where application output shares an inseparable control root. Avoid recursive self-capture.
- [ ] Stop all attempted units before collecting. Preserve the views until collection
  and saving finish; release unit-held bind references before strict overlay unmount.
  Keep write-back failure separate from file-list availability and cleanup verification.
- [ ] Extend the native report with the shared filesystem result. Preserve existing
  recording checks, host `/dev/shm` observation and launch/machine snapshot purposes.
- [ ] Carry the existing ownership records into interrupted-run reset. Stop writers
  before discarding leftover views; never replay a partial write-back automatically.

**Validation**

- [ ] Real native units and typed programs see tracked views at their original targets.
  Writes remain isolated during execution and reach the original directories after write-back.
- [ ] One real Compose fixture and one real systemd fixture produce equivalent change
  records for equivalent writes, deletions, shared aliases and read-only aliases.
- [ ] A unit that writes relative paths into its declared working directory has those
  writes captured. A unit with no declared working directory runs unchanged.
- [ ] A unit behaves correctly with a view over its working directory, including units
  started under the Dora daemon.
- [ ] Cover source resolution, optional missing sources, drop-ins, nested binds, private
  roots, nontrivial filenames, output/control-root separation and explicit exclusions.
- [ ] Verify preparation rollback, cancellation before/during startup, active writers,
  report-save failure, busy mount, external source mutation and controller restart.
- [ ] Build unprivileged; elevate only capability-required test executables. Verify all
  owned mounts and units are gone after success and retained as specified after failure.

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

## Milestone 4: Keep a run's files with the run

**Work items**

- [ ] Add a keep-with-the-run disposition beside write back and discard in `host-filesystem`.
- [ ] In finalization, reuse the unmount and upper-layer inspection. Copy the transfer
  list into a folder inside the run and record the removal list. Leave the source untouched.
- [ ] Remove the temporary view folder only after the copy is verified. A failed copy
  preserves the upper data, as a failed write back does.
- [ ] Record the disposition and the kept location in the file-change report.
- [ ] Add one Console configuration field selecting write back or keep with the run,
  default write back. Pass it to Compose and systemd runs.
- [ ] Show the policy at Review and the kept location with the result. Add no file browser.
- [ ] Add a download action for each listed file inside the run folder: kept files and log
  dumps. Reuse the authenticated artifact route and the safe open beneath the run root.
  Send every file as an attachment.

**Validation**

- [ ] The kept files match the report's added and modified entries exactly.
- [ ] The source folder is byte-identical before and after a keep run.
- [ ] Deletions appear in the report and remove nothing.
- [ ] A second run starts from the original source and keeps its own separate files.
- [ ] Full disk and an interrupted copy preserve the upper data and block verified cleanup.
- [ ] The view folder and the run folder on different filesystems.
- [ ] A kept file and a log dump download intact. Paths outside the run folder, traversal
  and symlink escapes are refused. A kept HTML file downloads and never renders in Console.
- [ ] Projects with no setting behave as before.

## Integration and qualification

**Work items**

- [ ] Publish focused implementation PRs in the order above. Keep the shared extraction
  behavior-preserving and finish systemd capture before claiming backend parity.
- [ ] Record completed and deferred checks here as implementation lands.

**Validation**

- [ ] Run full local CI on the final implementation source revision at integration:
  boundaries, formatting, default/all-feature Clippy and workspace tests, storage without
  default features and snapshot synchronization. Use release/CI profiles and locked dependencies.
- [ ] Run ARM64 worker validation and report it separately from local results.
- [ ] Qualify packaged systemd mount visibility, OverlayFS backing and capabilities on
  a real target. Unit tests alone do not establish this deployment contract.
- [ ] Keep Holoscan, CUDA/GPU/IPC and sensor qualification explicit separate work.

Design validation: source/guide review, local links, SVG XML/rendering and whitespace checks pass.
Runtime behavior remains unverified; the unchecked milestone checks above define acceptance.
Runtime checks above are deferred until implementation; this design PR changes documentation only.
