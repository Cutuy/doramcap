# Overlays for writable mounts: implementation plan

Created November 11, 2026. Updated November 12, 2026: milestone 2 implemented.
Status: milestone 1 merged ([#318](https://github.com/Cutuy/doramcap/pull/318));
milestone 3 merged ([#320](https://github.com/Cutuy/doramcap/pull/320));
milestone 2 ([#326](https://github.com/Cutuy/doramcap/pull/326)) ready for review. Milestones 4 and 5 remain planned.
[Architecture](111026-file-change-report.md). Terms follow its [term list](111026-file-change-report.md#terms).

## Order

| Milestone | Starts | Reason |
| --- | --- | --- |
| 1: move the overlay code | Merged | Moves overlays and change records into `host-filesystem`; behavior unchanged. |
| 3: report in Console | Merged, after 1 | Reads change records, whatever produced them. |
| 2: change list from the uppers | Done, in review | Replaces the before/after folder walks; every later milestone builds on it. |
| 4: systemd overlays | After 2 | systemd starts on the final change list. |
| 5: keep with the run | After 2 and 3 | Keeps the uppers and uses milestone 3's download route. |

Milestone numbers 1 and 3 keep their earlier meaning, so PR references stay valid.

The [admission lock](https://github.com/Cutuy/doramcap/pull/315) and the
[shared apps crate's pieces](https://github.com/Cutuy/doramcap/pull/317) are merged.
The apps crate's [system apps sequences](https://github.com/Cutuy/doramcap/pull/324)
are in review; they and milestone 1 here both reshape the Compose run sequence. The
[Holoscan overview](https://github.com/Cutuy/doramcap/pull/264) holds the order
across all of this work.

## Milestone 1: Extract Linux capture and preserve Docker behavior

**Work items**

- [x] Move the existing overlay trees, mapping, direct upper actions, metadata
  traversal, record readers/writers and schema into `host-filesystem`. Keep this crate
  independent of Docker, workloads and `orchestrator`.
- [x] Put concrete shared workflow steps in `workload-lifecycle`. Extract the common
  capture/finalization sequence from Compose and use it without a plugin framework.
- [x] Keep Docker socket namespace verification, Engine layer diffs, volume discovery
  and owned-container checks in `docker-engine`. Supply their observations to the common report.
- [x] Observe stopped-container inspection, layer diffs and volume re-resolution in
  one Docker workflow step. Preserve its producer dependency into shared collection.
- [x] Replace Docker-specific mount input types at the shared boundary with generated
  source/consumer/access records. Keep propagation and SELinux validation in the Docker adapter.
  New source/observation strings use empty values for absent paths and reasons.
- [x] Preserve `doramcap.filesystem.v1` record tags, path bytes, existing category values,
  overlay/checkpoint layouts, reset behavior, write-back/discard and Compose step lineage.
  Update owners, public names, callers, API doc comments and boundary rules together; add no forwarding aliases.
- [x] Reuse the repository's existing dependencies and implementations. No new external
  crate or custom capture algorithm is selected by this plan.

**Validation**

- [x] Existing real OverlayFS cases pass after the move: isolation, shared aliases,
  additions, modifications, deletion/opaque directories, write-back and discard.
- [x] Preserve busy-mount, full-disk, external-lower-change and interrupted-apply behavior.
- [x] Existing Compose file-change manifests and independent readback remain compatible.
- [x] Boundary checks, focused release tests, formatting and strict Clippy pass with locked dependencies.

## Milestone 2: Read the change list from the uppers

**Work items**

- [x] In `host-filesystem`, read each upper into change records: added when absent from
  the lower, deleted when the run removed it, and modified when its type, permissions,
  owner or modification time differs from the lower, or its size for a file or symlink.
  Ignore a folder's size. Leave out entries that match the lower; a folder that gained or
  lost an entry has a new modification time and is listed. Read no file contents. Take "before"
  metadata from the lower and "after" from the upper. Use the one upper reader that
  write back already uses.
- [x] Record one deleted entry for each path under a deleted folder in the lower.
- [x] Record sockets, FIFOs, device files and hard links like any other entry. Remove the
  reader's refusals for them.
- [x] List each output folder directly, every file as added. Keep it mounted directly.
  Skip Console's own records; mark a mount of the whole run folder as partly listed.
- [x] Mount unsupported sources directly with a coverage reason: writable single files,
  pseudo-filesystems and sources containing mountpoints. Preserve native host-shared-memory
  passthrough and its separate observation contract. Reject unsafe direct/overlay overlaps.
- [x] Report Docker named volumes as not tracked.
- [x] Write back: skip sockets, FIFOs and device files; copy a new hard link as a separate
  file; copy a modified file that has other names. Add one warning per such entry to the
  report's write-back result. Keep the run outcome and the change list unaffected.
- [x] Delete the before/after folder walks, their records and the metadata comparison,
  for host folders and named volumes. Delete the check that the host folder stayed
  unchanged and the hard-link walk before launch. Keep Docker's container filesystem diff.
- [x] Let the caller name an overlay directory after its host folder.
- [x] Read uppers without requiring a mount. Order the run as: stop apps, read uppers,
  save the report, unmount, apply the write policy, clean up.
- [x] Preserve the `doramcap.filesystem.v1` binary tags, categories, manifests and report
  checkpoint. Use their generated types directly across collection and Console.

**Validation**

- [x] Native changes and metadata records.
- [x] Unchanged copy-up omission.
- [x] Special nodes and hard-link write-back.
- [x] Independent before/after comparison.
- [x] Output retention and private records.
- [x] Review/result coverage consistency.
- [x] Large unchanged folder timings.
- [x] Concurrent host edits.
- [x] Discard and failure recovery.
- [x] Library boundaries, formatting and lint.
- [x] Full local workspace validation.
- [x] ARM64 worker validation.

### PR 326 repair

**Work items**

- [x] Preserve live host shared memory by filesystem identity, including hugetlbfs and `/dev/shm` aliases.
- [x] Reject direct/overlay overlaps before mounting; share supported parent/child views.
- [x] Use typed source classifications for execution and coverage; share alias resolution with Review.
- [x] Pass private record paths from their owners, including per-app inputs and caller records.
- [x] Update filesystem contracts and convert the edited Compose runner to TypeScript.

**Validation**

- [x] Host shared-memory identity and aliases.
- [x] Unsafe overlap rejection.
- [x] Supported parent/child sharing.
- [x] Owner-supplied record exclusions.
- [x] Console persistence and restart export.
- [x] Console bindings, server and frontend.
- [x] TypeScript runner checks.
- [x] Final local and ARM64 checks.

The native `upper_records_match_walk_reference_and_measure_unchanged_tree_listing`
fixture compares upper records with an independent before/after walk. On local
x86_64 in release mode, 20,000 unchanged files measured about 65 ms preparation,
0.23 ms unchanged-tree listing and 3.74 ms changed-upper listing. Run it through
`cargo test --release --locked -p host-filesystem --test overlay --no-run`, then
elevate only the built test executable with that test name.

## Milestone 3: Persist one report and expose preview plus JSON Lines

**Work items**

- [x] Use shared generated types for category counts, source coverage, report state
  and run-relative manifest references. Preserve absence separately from zero changes.
- [x] Carry the result through `ExecutionResult` and the Console run result. Keep full
  lists and unbounded source notes out of progress events and live status. Systemd
  results read as unavailable until milestone 4.
- [x] Sync every manifest and its folder, then atomically publish `filesystem/report.pb`
  before destructive finalization. Save its Console reference with the existing
  marker/History transaction. A crash before publication cannot establish a completed report.
- [x] Add the file-change summary and artifact reference to `console-report.json`.
  Read the saved report for current and past results; do not reconstruct it from progress.
- [x] Save an 8 KiB encoded preview with the run, including shown/total counts and
  incomplete-observation state. No capture limit; individual records are never cut.
- [x] Add a `file-changes` artifact kind to the authenticated download route. Stream
  JSON Lines from saved manifests with backpressure and descriptor cleanup on disconnect.
- [x] Export category summaries and full coverage notes alongside every change, then a
  completion record. Abort on malformed/truncated records or per-kind count mismatch.
- [x] Export 64-bit values as decimal strings and raw path/symlink bytes as base64 with
  readable display fields. Render paths as text.
- [x] Show a File changes section with counts, coverage, preview and Download file changes,
  reused for History. Availability is independent of recording finalization.
- [x] Keep recording-byte totals limited to declared recordings. Preserve old-run decoding,
  artifact authorization and existing filenames.
- [x] The next Prepare waits until the previous run's result, including its preview, is saved.

**Validation**

- [x] Result and saved-report consistency.
- [x] Large-list export and preview bounds.
- [x] Missing, malformed and incomplete data.
- [x] Run confinement, authorization and download cancellation.
- [x] Compose report persistence after restart.
- [x] Failed-run service recovery.
- [ ] Failed Compose run after restart.
- [ ] Real systemd runs.
- [ ] Desktop and narrow-screen rendering.

## Milestone 4: Give systemd services the same overlays

Planned in the [systemd overlays plan](../../libs/workload-systemd/docs/111326-systemd-overlays-impl.md).

## Milestone 5: Keep a run's files with the run

Planned in the [keep files plan](111326-keep-files-with-run-impl.md).

## Milestone 6: Writable folders at Review

**Work items**

- [x] Render shared source handling, app associations and the project write policy.
- [x] Offer copyable configuration lines without editing settings.
- [x] Save the reviewed rows with the run for Result and restart.

**Validation**

- [x] Frontend handling states and read-only controls.
- [x] Review rows preserved in saved Result.
- [x] Desktop and narrow Chromium views and Copy.

## Integration and qualification

**Work items**

- [ ] Publish focused implementation PRs in the order above.
- [ ] Record completed and deferred checks here as implementation lands.

**Validation**

- [ ] Run full local CI on the final implementation source revision at integration:
  boundaries, formatting, default/all-feature Clippy and workspace tests, storage without
  default features and snapshot synchronization. Use release/CI profiles and locked dependencies.
- [ ] Run ARM64 worker validation and report it separately from local results.
- [ ] Qualify systemd's sight of overlays, OverlayFS backing and capabilities on a real
  packaged target. Unit tests alone do not establish this deployment contract.
- [ ] Keep Holoscan, CUDA/GPU/IPC and sensor qualification explicit separate work. The
  Holoscan runner checks that an extra data folder appears at the same path with an
  overlay directory as without one.
