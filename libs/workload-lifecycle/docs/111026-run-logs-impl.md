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
- [x] Forward resolved diagnostics to Review's existing warning display.
- [x] Keep policy and qualification in `compose-apps` after its extraction on main.
- [x] Document shared Docker rate limiting, host retention and measured loss.

**Validation**

- [ ] Recheck the logging matrix, native Review regression and focused release
  checks after rebasing onto current main.
- [ ] Recheck real Docker driver settings, both failure-output streams and
  post-removal journal lookup after the rebase.
- [ ] Recheck low-rate, burst and saturation output with a quiet container after
  the rebase; record loss as a measurement.
- [ ] Run ARM64 validation at this integration point and record it separately.

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
