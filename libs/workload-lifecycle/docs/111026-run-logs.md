# Application logs for a run

Created November 10, 2026. Updated November 11, 2026. Tracking note; direction agreed.

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

## Open questions

- How entries are matched to a run: unit names, container labels and the run's time window.
- Whether the dump has a size limit per program.
- The journal drops lines from very chatty services by default. Decide whether Console reports that.
- How Console reads the journal. Candidate libraries need review before adoption.
- Confirm on a real container that the journal driver works as expected, including
  reading back the last lines for the failure message.
