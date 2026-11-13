# Journal export: implementation plan

Created November 13, 2026. Status: implementation in progress.
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

- [ ] Add an export step in `workload-lifecycle` that takes a program list (ID and
  journal match) and the run folder.
- [ ] Ask the journal to finish writing, then run `journalctl` once per program with its
  match and a timestamped plain text output. Stream into `logs/<n>.log`.
- [ ] Write `logs/index.pb` from a new protobuf message: program ID, identity, state
  (exported, skipped, failed), reason, entry count and size.
- [ ] Report export failure separately from the run outcome; keep written files.

**Validation**

- [ ] Adjacent runs with the same unit name read only their own entries.
- [ ] Journal unavailable, retention loss and run storage full.
- [ ] Files survive refresh and restart.

## Milestone 2: Identities from both workflows

**Work items**

- [ ] systemd: read each unit's invocation ID after start and save it with the run's
  ownership records. Match its own output and the service manager's entries for it.
- [ ] Compose: save each container's full container ID; skip containers with
  project-managed logging and list them as skipped.
- [ ] Call the export step after every program stops, before cleanup finishes.
- [ ] Apply the agreed choice for export after a Console crash.

**Validation**

- [ ] Program failure, stop and cancellation still export.
- [ ] Controller restart and interrupted export.
- [ ] Measure identity loss for programs that exit almost at once.

## Milestone 3: Logs in Console

**Work items**

- [ ] Show a Logs list on Result and History: program, state and size.
- [ ] Describe an exported file as the entries still available in the journal.
- [ ] Add downloads through the run-file route from the
  [keep files plan](https://github.com/Cutuy/doramcap/pull/354), whichever lands second.

**Validation**

- [ ] Logs list after restart.
- [ ] Desktop and narrow-screen rendering.

## Tests

Journal-dependent tests run on a real host and elevate only the test executable.
Scripted end-to-end checks are TypeScript.
