![Shared run sequence](111026-supervisor-run-driver.svg)

# Retire Supervisor's legacy run driver

Created: 2026-11-10. [Implementation plan](111026-supervisor-run-driver-impl.md).

Compose, systemd and Supervisor use the same `run::run` function with backend-owned
`Running` handles. Supervisor’s former lifecycle driver is removed. This follows the review of
[PR #252](https://github.com/Cutuy/doramcap/pull/252).

The shared driver owns sequencing: start, readiness, observation, stop, recording
checks and removal. Supervisor owns its daemon, monitor, durable ownership ledger,
process-exit notifications and cleanup evidence. Its existing observed backend
steps remain the machine-operation boundary.

| Contract | Behavior to preserve |
| --- | --- |
| Startup and stop | Partial startup remains owned; cooperative stop awaits teardown; stop before launch suppresses readiness. |
| Readiness | A closed readiness receiver ends execution and triggers teardown. |
| Observation | Either daemon or monitor exit ends the recording attempt through pidfd notifications. |
| Recordings | Forced shutdown cannot establish recording success, even after writers stop. |
| Outcomes | Execution, recording validity and verified cleanup remain independent. |
| Ownership | Dropping a run keeps admission blocked; cleanup retains identity checks and the durable ledger. |

The shared driver checks recording files after writers stop. Supervisor combines
that file result with completed-launch and unforced-shutdown evidence in its
report, following Compose’s existing report policy. Forced shutdown still rejects
recording success when the MCAP footer is valid; verified cleanup remains independent.
This introduces a read-only recording-check step after forced or incomplete launches.
No recording-policy hook is added to `Running`.

`Running::ready` can return an execution error so Supervisor retains teardown on a
closed readiness receiver. Compose and systemd retain their readiness behavior.
The current `MonitorRun`, `Ownership` and `RunReport` formats and app placement remain.

Limit the migration to the driver and its callers. Transport changes, monitor
collectors and new metrics remain separate work.
