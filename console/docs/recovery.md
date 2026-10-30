# Recover blocked admission

The admission marker is server state. Browser reloads and cache clearing preserve
it. A blocked run can still have a valid CPU recording.

A host administrator must perform recovery with the Console stopped:

1. Save available downloads. Inspect `data_root/runs/<run_id>/report.json`, the
   filesystem evidence and overlay results. These private files may contain
   sensitive host details; do not publish them.
2. Verify Docker containers, networks and volumes for `io.doramcap.run=<run_id>`
   and the recorded Compose projects. Resolve remaining owned resources and
   incomplete filesystem disposition. Confirm overlay mounts are gone in the
   owning mount namespace. Preserve retained data until its disposition is resolved.
3. When the boundary is verified clean, archive `data_root/current.pb` in the
   existing run artifact directory as `admission-before-manual-recovery.pb`.
   Preserve the original report and its unverified cleanup outcome. Record what
   was checked and resolved beside it. Do not overwrite an existing recovery record.
4. Restart with the same configuration and data root. Project selection is
   available again. Historical evidence remains in the run artifact directory.

If cleanup cannot be established, keep admission blocked. Forced process exit
alone does not prove that resources or mounts are gone. See the
[filesystem recovery constraints](../../libs/workload-docker/docs/102626-docker-experiment-isolation.md#harvest-semantics-and-failure-handling).
