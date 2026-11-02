# Interrupted runs

The Compose workflow owns repository admission and reset. Starting a run acquires
that repository's controller lease and resets its old containers and overlay
workspace before preparation. Reset discards interrupted upper-layer changes;
it does not replay unfinished writes.

Console's marker stores request and display metadata. An interrupted or
invalid marker returns Console to project selection. It does not authorize
cleanup or bypass an active repository controller. Reset failures remain visible
and prevent launch until the underlying failure is resolved.

Completed reports and declared recordings remain under the private data root.

`console.db` holds the marker and every run's saved status. Console refuses to start
if it cannot open it, and refuses a run it cannot record. A run still open when
Console stops becomes **Interrupted**, with no finish time and no download. If the
file is corrupt, stop Console and move `console.db*` aside. Past runs then leave
History, and their recordings stay under `runs/`. Detailed design:
[experiment history](110226-experiment-history.md).

See the [workflow contract](../../libs/workload-docker/docs/102926-shared-dora-experiment.md).
