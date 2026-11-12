# Overlays for writable mounts: implementation plan

Created November 11, 2026. Updated November 11, 2026: decisions from the plan review recorded.
Status: milestone 1 implemented in review ([#318](https://github.com/Cutuy/doramcap/pull/318)),
milestone 3 implemented in review ([#320](https://github.com/Cutuy/doramcap/pull/320)), the rest planned.
[Architecture](111026-file-change-report.md). Terms follow its [term list](111026-file-change-report.md#terms).

## Order

| Milestone | Starts | Reason |
| --- | --- | --- |
| 1: move the overlay code | Done, in review | Moves overlays and change records into `host-filesystem`; behavior unchanged. |
| 3: report in Console | Done, in review, after 1 | Reads change records, whatever produced them. |
| 2: change list from the uppers | After 1 | Replaces the before/after folder walks; every later milestone builds on it. |
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

- [ ] Move the existing overlay trees, mapping, direct upper actions, metadata
  traversal, record readers/writers and schema into `host-filesystem`. Keep this crate
  independent of Docker, workloads and `orchestrator`.
- [ ] Put concrete shared workflow steps in `workload-lifecycle`. Extract the common
  capture/finalization sequence from Compose and use it without a plugin framework.
- [ ] Keep Docker socket namespace verification, Engine layer diffs, volume discovery
  and owned-container checks in `docker-engine`. Supply their observations to the common report.
- [ ] Replace Docker-specific mount input types at the shared boundary with generated
  source/consumer/access records. Keep propagation and SELinux validation in the Docker adapter.
- [ ] Preserve `doramcap.filesystem.v1` record tags, path bytes, existing category values,
  overlay/checkpoint layouts, reset behavior, write-back/discard and Compose step lineage.
  Update owners, public names, callers, API doc comments and boundary rules together; add no forwarding aliases.
- [ ] Reuse the repository's existing dependencies and implementations. No new external
  crate or custom capture algorithm is selected by this plan.

**Validation**

- [ ] Existing real OverlayFS cases pass after the move: isolation, shared aliases,
  additions, modifications, deletion/opaque directories, write-back and discard.
- [ ] Preserve busy-mount, full-disk, external-lower-change and interrupted-apply behavior.
- [ ] Existing Compose file-change manifests and independent readback remain compatible.
- [ ] Boundary checks, focused release tests, formatting and strict Clippy pass with locked dependencies.

## Milestone 2: Read the change list from the uppers

**Work items**

- [ ] In `host-filesystem`, read each upper into change records: added when absent from
  the lower, deleted when the run removed it, and modified when its type, permissions,
  owner or modification time differs from the lower, or its size for a file or symlink.
  Ignore a folder's size. Leave out entries that match the lower. Read no file contents. Take "before"
  metadata from the lower and "after" from the upper. Use the one upper reader that
  write back already uses.
- [ ] Record one deleted entry for each path under a deleted folder in the lower.
- [ ] Record sockets, FIFOs, device files and hard links like any other entry. Remove the
  reader's refusals for them.
- [ ] List each output folder directly, every file as added. Keep it mounted directly.
  Skip Console's own records; mark a mount of the whole run folder as partly listed.
- [ ] Mount directly, and report as not tracked with a reason, each writable mount that is
  a single file, sits under `/proc`, `/sys` or `/dev`, or contains another mount point.
- [ ] Report Docker named volumes as not tracked.
- [ ] Write back: skip sockets, FIFOs and device files; copy a new hard link as a separate
  file; copy a modified file that has other names. Add one warning per such entry to the
  report's write-back result. Keep the run outcome and the change list unaffected.
- [ ] Delete the before/after folder walks, their records and the metadata comparison,
  for host folders and named volumes. Delete the check that the host folder stayed
  unchanged and the hard-link walk before launch. Keep Docker's container filesystem diff.
- [ ] Let the caller name an overlay directory after its host folder.
- [ ] Read uppers without requiring a mount. Order the run as: stop apps, read uppers,
  save the report, unmount, apply the write policy, clean up.
- [ ] Keep the `doramcap.filesystem.v1` change records, categories, manifests and report
  checkpoint unchanged, so milestone 3 needs no change.

**Validation**

- [ ] Real OverlayFS fixtures: additions, modifications, deletions, a deleted folder with
  contents, renames, symlinks, non-UTF-8 names and permission-only changes produce the
  expected records.
- [ ] A file opened for writing with nothing written, and the folders above a changed
  file, are absent from the change list on a filesystem where an upper folder's size
  differs from the lower folder's.
- [ ] A socket, a FIFO, a new hard link and a modified file with two names each appear in
  the change list. Write back finishes, the host folder holds the expected result and the
  report carries one warning per entry.
- [ ] The same Compose writes to a host folder produce the same records as the walk-based
  list did, apart from entries that match the lower.
- [ ] An output folder's files are listed as added and remain in the run folder after an
  interrupted run. Console's own records are absent from the list.
- [ ] A single-file mount, a mount containing another mount point and a named volume start
  normally and appear as not tracked at Review and in the result.
- [ ] Listing cost follows the size of the upper: a large unchanged host folder lists
  quickly, and preparing it walks nothing.
- [ ] A change made to the host folder during the run, on a path the run also wrote, ends
  with the run's version after write back.
- [ ] Discard, busy overlays, full disk and interrupted runs behave as before.
- [ ] Boundary checks, focused release tests, formatting and strict Clippy pass with locked dependencies.

## Milestone 3: Persist one report and expose preview plus JSON Lines

**Work items**

- [ ] Use shared generated types for category counts, source coverage, report state
  and run-relative manifest references. Preserve absence separately from zero changes.
- [ ] Carry the result through `ExecutionResult` and the Console run result for Compose and
  systemd. Keep full lists and unbounded source notes out of progress events and live status.
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

- [ ] First, on a real systemd host, confirm the service manager sees overlays Console
  mounts and that an owned drop-in can replace one `BindPaths=` entry while keeping the
  others and their order. Stop and report if either fails.
- [ ] Read each unit's writable host folders: from the app's settings for services
  Console creates, and from the unit file and its drop-ins through the service manager
  for your own unit files. Reuse the existing input copies; add no unit-file parser.
- [ ] Mount one overlay per writable host folder before any unit starts, through the
  shared lifecycle. Point each mount at its overlay directory with an owned property or
  drop-in, keeping the path the app sees and its read-only or writable setting.
- [ ] Delete the mount Console adds for a created service's undeclared working directory.
- [ ] Check that each declared working directory exists where its app will run: when
  Review is produced and again before anything is mounted or started. Fail with the app
  and the directory. Skip units that write `WorkingDirectory=-/path`.
- [ ] At Review, list a writable working directory outside every writable mount as not
  tracked and suggest declaring it. List your own unit files' other writes as not tracked.
- [ ] Add a test that the Dora coordinator and daemon keep `/runtime` under their
  declared output folder.
- [ ] Stop all started units and release their hold on the overlays before unmounting.
  Then reuse milestone 2's change list and the shared write policy.
- [ ] Extend the systemd run report with the shared file-change result. Keep recording
  checks, host `/dev/shm` observation and machine snapshots unchanged.
- [ ] Carry ownership records into interrupted-run reset: stop units, then discard
  leftover overlays.

**Validation**

- [ ] Real created services and unit files see their overlay directories at the paths
  they expect; writes stay out of the host folder during the run and reach it after write back.
- [ ] Equivalent Compose and systemd fixtures produce equivalent change records.
- [ ] A missing working directory fails at Review and again at Start with nothing mounted
  or started. `WorkingDirectory=-/path` starts. Dora's system apps start unchanged.
- [ ] Preparation rollback, cancellation before and during startup, busy overlays,
  report-save failure and Console restart.
- [ ] Build unprivileged; elevate only capability-required test executables. All owned
  mounts and units are gone after success and kept as specified after failure.

## Milestone 5: Keep a run's files with the run

**Work items**

- [ ] Add keep with the run beside write back and discard in `host-filesystem`: copy each
  upper's files into the run folder and record its deletions. Leave the host folder untouched.
- [ ] Remove the upper only after the copy is verified. A failed copy keeps the upper, as
  a write back stopped by an I/O error does.
- [ ] Record the policy and the kept location in the file-change report.
- [ ] Add one Console setting selecting write back, keep with the run or discard, default
  write back. Pass it to Compose and systemd runs.
- [ ] Show the policy at Review and the kept location with the result. Add no file browser.
- [ ] Add a download action for kept files, output folder files and log dumps as one new
  kind on the authenticated artifact route. The request names a saved change-list entry
  or a program, never a path. The server opens the file only inside the run's kept-files
  folder, the entry's output folder or the log folder, by its raw name bytes. Send every file as an attachment with an encoded filename.

**Validation**

- [ ] The kept files match the added and modified entries of overlaid host folders
  exactly. Output folder entries stay in their output folder and are copied nowhere.
- [ ] The host folder is byte-identical before and after a keep run.
- [ ] Deletions appear in the report and remove nothing.
- [ ] A second run starts from the unchanged host folder and keeps its own separate files.
- [ ] Full disk and an interrupted copy keep the upper and block verified cleanup.
- [ ] The upper and the run folder on different filesystems.
- [ ] A kept file, one with a non-UTF-8 name, an output folder file and a log dump
  download intact. Requests for
  Console's run records, another run's entries, traversal and symlink escapes are refused.
  A kept HTML file downloads and never renders in Console.
- [ ] Under discard the host folder is byte-identical before and after, the run keeps the
  change list and no kept files.
- [ ] Output folder files stay in the run folder under all three policies.
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
- [ ] Qualify systemd's sight of overlays, OverlayFS backing and capabilities on a real
  packaged target. Unit tests alone do not establish this deployment contract.
- [ ] Keep Holoscan, CUDA/GPU/IPC and sensor qualification explicit separate work. The
  Holoscan runner checks that an extra data folder appears at the same path with an
  overlay directory as without one.

Design validation: source/guide review, local links, SVG XML/rendering and whitespace checks pass.
Runtime checks above are deferred until implementation; this design PR changes documentation only.
