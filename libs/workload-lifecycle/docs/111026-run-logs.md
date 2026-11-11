# Application logs for a run

Created November 10, 2026. Updated November 11, 2026. Log dump design agreed; implementation not started.

Console does not keep application output with a run today. Every workflow sends
output to the host journal, and Console dumps each run's output into the run's
files. Keeping those files is then handled with the run's other files, in
[Cutuy/doramcap#316](https://github.com/Cutuy/doramcap/pull/316).

## Today

| Workflow | Where application output goes | After the run | What Console reads |
| --- | --- | --- | --- |
| systemd | The host journal, by systemd's default. | Stays in the journal under the unit's name while the host keeps it. | Nothing. A unit's exit reports result, exit code, CPU and memory. |
| Compose | Docker's per-container log. | Deleted when Console removes the container. | The last 20 lines of a failed container, for the error message. |
| Holoscan (planned) | Agreed: the host journal. | Stays in the journal. | The last lines of a failed container. |

## Agreed direction

- Application output from every workflow goes to the host journal.
- Containers use Docker's journal log driver. Holoscan sets it as a launch option.
  Compose would set it in the manifest Console already rewrites.
- The user's choice wins. A service or project that configures its own log driver
  keeps it. Review shows that the log is set by the project, and Console collects
  nothing for that container.
- Console's per-container setting takes precedence over a host-wide Docker default.
- An application that writes its own log files is unaffected. Those files appear
  in the file-change report when they land in a captured folder.

## Planned work

| Item | Description |
| --- | --- |
| Compose through the journal | Set the journal log driver for services without their own logging. |
| Log dump | At the end of a run, write each program's full output from the journal into a run-owned output folder, one file per unit or container. |
| Web app | Show the dumped logs with the run's other files. |

The dump is a set of ordinary run files. The file-change report lists it and the
run folder keeps it, so a run's log no longer depends on how long the host keeps
its journal. Console keeps the full output and selects no snippets.

## Log dump

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
service manager reports a unit's current one. The container half follows
Docker's documentation and is untested.

## Open questions

- A process that exits almost at once can lose its identity in the journal, so
  its last lines may be missing from the dump. Measure how often this occurs.
- The journal drops lines from very chatty services by default. Decide whether Console reports that.
- Confirm on a real container that the journal driver works as expected, including
  reading back the last lines for the failure message.
