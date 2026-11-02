# Interrupted runs

The Compose workflow owns repository admission and reset. Starting a run acquires
that repository's controller lease and resets its old containers and overlay
workspace before preparation. Reset discards interrupted upper-layer changes;
it does not replay unfinished writes.

Console's `current.pb` stores request and display metadata. An interrupted or
invalid marker returns Console to project selection. It does not authorize
cleanup or bypass an active repository controller. Reset failures remain visible
and prevent launch until the underlying failure is resolved.

Completed reports and declared recordings remain under the private data root.

`console.db` is an optional index of past runs. Runs never depend on it: a failure
shows as degraded history and the next open rebuilds missing runs from
`runs/*/activity.pb` and `current.pb`. A run still open when Console stops becomes
**Interrupted**, with no finish time and no download. If the file is corrupt, stop
Console and move `console.db*` aside; Console never replaces it. Detailed design:
[experiment history](110226-experiment-history.md).

See the [workflow contract](../../libs/workload-docker/docs/102926-shared-dora-experiment.md).
