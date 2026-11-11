# Application logs: implementation

Created November 11, 2026. Implements [option A in the architecture](111026-run-logs.md).

The host journal captures output under its existing rate limits and retention.
Console's later export reads entries still available. Capture is best effort;
the implementation leaves installer and host settings unchanged.

## Milestone 1 — Compose routing and Review

**Work items**

- [x] Set `logging.driver: journald` in the frozen Compose manifest only when the
  project omits `logging`; record the policy change.
- [x] Preserve every explicit logging block, including empty and options-only
  blocks; diagnose project-managed logging and its exclusion from collection.
- [x] Forward project-managed logging diagnostics to Review's existing warning
  display; preserve the existing handling of other Compose notes.
- [x] Keep policy and qualification in `compose-apps` after its extraction on main.
- [x] Document shared Docker rate limiting, host retention and measured loss.

**Validation**

- [x] Recheck the logging matrix, native Review regression and focused release
  checks after rebasing onto current main.
- [x] Recheck real Docker driver settings, both failure-output streams and
  post-removal journal lookup after the rebase.
- [x] Recheck low-rate, burst and saturation output with a quiet container after
  the rebase; record loss as a measurement.
- [x] ARM64 job 670 passed for this integration. Worker snapshot:
  `/home/doramcap-ci/jobs/5e6b1204a695424284a84713f441fb43`.

Validated Rust runtime/test source: `02ad06d3`, rebased onto main `3eb0381f`.
Release results: `compose-apps` 65 passed / 3 ignored; `workload-docker` 18 passed;
focused Clippy, formatting and library boundaries passed. Console typecheck and
server tests passed with pinned Node 26.13.0: 119 passed / 1 skipped. Both opt-in
Docker/journal tests passed as measurements; the latest saturation run retained
22/60 quiet lines per stream and lost all quiet markers after the noisy exit.
Architecture SVGs were rendered and local document links checked.

ARM64 job 670 passed default/all-feature workspace Clippy and tests, storage
without default features, native resource/supervisor checks and the worker's
other checks. The later Review-only filter in `34cfa1a3` passed the Console
typecheck and 119 server tests on pinned Node 26.13.0. Rust source is unchanged
by that follow-up. Full local workspace CI was not run. The remaining
journal-export and run-file checks belong to deferred milestone 2.

Earlier validation and counts are in the
[qualification record](111026-run-logs.md#journal-load-qualification). A passing
load measurement does not establish complete capture.

## Milestone 2 — Journal export and run files, deferred

Depends on [#316 milestone 4](https://github.com/Cutuy/doramcap/pull/316).

**Work items**

- [ ] After programs stop, ask the journal to finish writing and use `journalctl`
  to export all matching retained entries. Reuse the platform tool.
- [ ] Match each systemd start by its invocation ID, including service-manager
  entries carrying that invocation under their own field. Read the ID from the
  service manager after starting the unit.
- [ ] Match each container by its full container ID. Skip project-managed logging.
- [ ] Write one plain text file per program, with timestamps, into run-owned
  output storage. Apply the run folder's limits; add no separate per-program cap.
- [ ] Show exported files with the run's other files. Describe successful export
  as reading available journal entries; complete application output remains unknown.
- [ ] Preserve available output on read, write or finalization failure and report
  that failure separately from the application's result.

**Validation**

- [ ] Verify adjacent runs cannot read each other's output, including reused unit
  names and program failure.
- [ ] Measure identity loss for programs that exit almost immediately.
- [ ] Cover stop, cancellation, controller restart and interrupted export.
- [ ] Cover unavailable journal access, retention loss and run-storage exhaustion.
- [ ] Verify saved files survive refresh/restart and follow run-file retention.

No journal export or run-file integration is implemented by PR #314.
