# Journal export: implementation plan

Created November 13, 2026. Updated November 14, 2026. Status: implemented.
[Architecture](111326-journal-export.md). Replaces milestone 2 of the
[run logs plan](111026-run-logs-impl.md).

## Starting point

| Piece | Where |
| --- | --- |
| Compose output to the journal | `compose-apps`, from [Cutuy/doramcap#314](https://github.com/Cutuy/doramcap/pull/314); project-managed logging recorded at Review |
| systemd output to the journal | systemd default |
| Unit start | `workload_systemd::manager` `start_program` and `start_adopted` |
| Run end | `workload_lifecycle::run` with the `Running` trait |

## Milestone 1: Export step and run files

**Work items**

- [x] Add an export step in `workload-lifecycle` that takes a program list (ID and
  journal match) and the run folder.
- [x] Ask the journal to finish writing, then run `journalctl` once per program with its
  match and a timestamped plain text output. Stream into `logs/<n>.log`.
- [x] Write `logs/index.pb` from a new protobuf message: program ID, identity, state
  (pending, exported, skipped, failed), reason, entry count and size.
- [x] Report export failure separately from the run outcome; keep written files.

**Validation**

- [x] Adjacent runs with the same unit name read only their own entries.
- [x] Journal unavailable, retention loss and run storage full.
- [x] Files survive refresh and restart.

## Milestone 2: Identities from both workflows

**Work items**

- [x] systemd: read each unit's invocation ID after start and save it in the single
  `logs/index.pb`. Match its own output and the service manager's entries for it.
- [x] Compose: save each container's full container ID; skip containers with
  project-managed logging and list them as skipped.
- [x] Call the export step after every program stops, before cleanup finishes.
- [x] Apply the agreed choice for export after a Console crash.

**Validation**

- [x] Program failure, stop and cancellation still export.
- [x] Controller restart and interrupted export.
- [x] Measure identity loss for programs that exit almost at once.

## Milestone 3: Logs in Console

**Work items**

- [x] Show a Logs list on Result and History: program, state and size.
- [x] Describe an exported file as the entries still available in the journal.
- [x] Download program logs through the shared run-file route from the
  [keep files plan](https://github.com/Cutuy/doramcap/pull/354).
- [x] Select saved index positions and derive readable attachment names from program IDs.

**Validation**

- [x] Logs list after restart.
- [x] Desktop and narrow-screen rendering.
- [x] Duplicate program IDs remain selectable.
- [x] Readable attachment names and file confinement.
- [x] Rejected preparation after verified reset.

## Milestone 4: Owner replay and compatibility

**Work items**

- [x] Record the owner-approved behavior in the architecture.
- [x] Define typed Docker label metadata and explicit flat-label encoding/decoding.
- [x] Preserve existing label keys, string values and missing-field semantics.
- [x] Replace the separate index cutoff with existing encoded-message budgets.

**Validation**

- [x] Previous-version label maps and absent policy flags.
- [x] Encoded History, response and live-envelope boundaries.
- [ ] Native reset followed by preparation rejection.
- [ ] Concurrent repositories and reset-time container discovery.
- [ ] ARM64 integration.

## Tests

Journal-dependent tests run on a real host and elevate only the test executable.
Scripted end-to-end checks are TypeScript.
