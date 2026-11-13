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
- [x] Forward project-managed logging warnings to Review's existing warning
  display; preserve the existing handling of other Compose notes.
- [x] Carry Review warnings in their own protobuf field; do not classify message text.
- [x] Reject journal-dependent services during Review and fresh Start preparation
  when the Docker driver, host namespace or journal connection cannot be verified.
- [x] Keep policy and qualification in `compose-apps` after its extraction on main.
- [x] Document shared Docker rate limiting, host retention and measured loss.
- [x] Remove the scripted Rust qualification tests; keep their recorded measurements.

**Validation**

- [x] Recheck the logging matrix, missing-driver rejection, journal loss between
  Review and fresh Start resolution, and preservation of explicit logging.
- [x] Verify native Review rejects omitted logging on an unsupported host before
  mutation; forward warnings independently of diagnostic wording.
- [x] Reproduce Compose's API negotiation and daemon-info requests in the fake
  Engine without consuming expected image/resource responses.
- [x] Verify real local Docker/journal preparation through the built Console
  native bridge, with no container launch requested.
- [x] Run focused Clippy, formatting, boundaries, typecheck and local-link checks.
- [x] ARM64 job 677 passed for runtime source `11df6f96`. Snapshot:
  `/home/doramcap-ci/jobs/b6fcfa84c5b54059ac657b51e394d719`.

Validated November 12, 2026. Local x86_64, locked dependencies and CI profile:
`compose-apps` 68 passed / 3 ignored; `workload-docker` 18 passed. Console server
and native-addon tests: 120 passed / 1 skipped on pinned Node 26.13.0.
Only the real-host inspection executable was elevated; host settings were unchanged.

ARM64 passed default/all-feature workspace Clippy and tests, storage without
default features, Node/Console checks, native resource/supervisor checks and
recording qualification. Job 674 exposed a test fixture missing the new field;
the correction is included in the passing job 677 source.
Full local workspace CI was not run; no hosted checks were listed for this head.

The earlier [journal-load measurements](111026-run-logs.md#journal-load-qualification)
remain evidence from their recorded host; they were not rerun. A passing measurement
does not establish complete capture. Journal-export and run-file checks remain
in milestone 2.

## Milestone 2 — Journal export and run files, follow-up

Planned in the [journal export plan](111326-journal-export-impl.md).

No journal export or run-file integration is implemented by PR #314.
