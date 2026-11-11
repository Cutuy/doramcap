# Application logs for a run

Created November 10, 2026. Updated November 11, 2026. Compose journal driver implemented in #314; log dump deferred until #316 milestone 4.

Console does not keep application output with a run today. This PR sends Compose
output to the host journal by default. A later change will dump each run's output
into the run's files after
[Cutuy/doramcap#316 milestone 4](https://github.com/Cutuy/doramcap/pull/316) provides
keep-with-the-run storage.

## Today

| Workflow | Where application output goes | After the run | What Console reads |
| --- | --- | --- | --- |
| systemd | The host journal, by systemd's default. | Stays in the journal under the unit's name while the host keeps it. | Nothing. A unit's exit reports result, exit code, CPU and memory. |
| Compose | The host journal by default; explicit project logging is preserved. | Journal entries survive container removal while the host keeps them. | The last 20 lines of a failed container through Docker’s logs API, for the error message. |
| Holoscan (planned) | Agreed: the host journal. | Stays in the journal. | The last lines of a failed container. |

## Agreed direction

- Application output from every workflow goes to the host journal.
- Containers use Docker's journal log driver. Holoscan sets it as a launch option.
  Compose sets it in the frozen manifest shared by inspection and launch.
- The user's choice wins. An explicit `logging` block stays intact, including its
  options, an options-only block or an empty block. Those services retain Docker’s
  native behavior. Review shows that logging is configured by the project and
  Console will not collect its logs. The later log dump will skip these containers.
- Console's per-container setting takes precedence over a host-wide Docker default.
- An application that writes its own log files is unaffected. Those files appear
  in the file-change report when they land in a captured folder.

## Planned work

| Item | Description |
| --- | --- |
| Compose through the journal | Implemented in #314: set `journald` for services without a `logging` block; record the default in resolution policy changes. |
| Log dump | Deferred until #316 milestone 4: write each program's journal output into a run-owned output folder at the end of a run, one file per unit or container. |
| Web app | Show the dumped logs with the run's other files. |

The planned dump will be a set of ordinary run files. The file-change report will
list it and the run folder will keep it, so a run's log will no longer depend on
host journal retention. Console will keep the full output and select no snippets.

## Later log dump

Implementation depends on #316 milestone 4. The decisions below describe the
agreed follow-up.

At the end of a run, after every program has stopped, Console writes each
program's output from the journal into the run folder.

| Detail | Decision |
| --- | --- |
| Reader | Console runs `journalctl`, systemd's own journal tool. It reads every journal format and is present wherever the journal is. No library is added. |
| Scope | Each program's own identity, with no time window. |
| Files | One plain text file per unit or container, each line with its timestamp. |
| Size | No limit per program. Limits come with the run folder's limits. |
| Before reading | Console asks the journal to finish writing. |

Scoping by identity:

| Program | Identity | Source |
| --- | --- | --- |
| systemd unit | Its invocation ID | systemd assigns a new one at every start and records it on each entry. Console reads it from the service manager after starting the unit. |
| Container | Its full container ID | Docker's journal driver records it on each entry. Console tracks the containers it owns. |

A unit name is reused by every run, so a name with a time window cannot separate
runs exactly. The service manager's own messages about a unit carry the
invocation ID under a second field. The dump matches both, so each file shows
the unit's start and end.

A container whose project sets its own log driver has no dump. The run records
that its log is set by the project.

Confirmed on one host with systemd 255: entries carry the invocation ID and the
service manager reports a unit's current one. The real Compose test also confirms
that `journald` captures stdout and stderr, Docker's failure-output reader works,
and entries remain queryable by full container ID after container removal.
Explicit `json-file` logging and its options remain intact.

## Open questions

- A process that exits almost at once can lose its identity in the journal, so
  its last lines may be missing from the dump. Measure how often this occurs.
- The journal drops lines from very chatty services by default. Decide whether Console reports that.
