# systemd overlays: implementation plan

Created November 13, 2026. Updated November 14, 2026.
[Architecture and nine approved decisions](111326-systemd-overlays.md#approved-decisions).
Terms follow the [overlay term list](../../../console/docs/111026-file-change-report.md#terms).

## Mechanisms

`host_filesystem::overlay` resolves, prepares and finalizes directory views.
`workload_lifecycle::capture` owns the neutral filesystem steps. `workload-systemd`
resolves native binds through systemd and owns unit links, drop-ins and cancellation.
Console carries `SystemdRunReport` execution errors and shared file-change references
into the durable run result.

Setup uses cooperative cancellation. Read-only inspection can stop immediately;
attempted writes, links, reloads and blocking mount work settle before the next
checkpoint. The shared overlay preparer checks cancellation between operations.
An independently owned setup task holds a kernel lock on the scope directory;
dropping its caller requests cancellation, and Cleanup waits for lock release.
Cleanup holds admission, stops owned services, verifies empty cgroups, discards any
partial overlay workspace and removes runtime files. Explicit Reset discards
abandoned writes; normal stopped runs use their selected write policy.

The owned `15-doramcap-overlays.conf` resets both native bind lists and rebuilds them
in order. `20-doramcap-shm.conf` adds the private-shm default afterward. Kernel mount
options use directory descriptors; reports retain host paths. Native failure signals
cover reserved systemd setup failures after a successful `Type=simple` start job;
ordinary app exits remain recorded until Stop. Capture `systemctl status`
before teardown to retain systemd's explanation without duplicating its exit-code table.

`SystemdPlan.discard_writes` remains an interim field in #357. Stacked #361 removes
it in favor of the filesystem-owned policy in `RunPlan.write_policy`.

## Milestone 1: Native mount contract

**Work items**

- [x] Qualify controller mounts visible to transient and adopted units.
- [x] Use one owned drop-in to rebuild writable and read-only native binds.
- [x] Preserve destinations, access modes, recursive flags, order and native specifiers.
- [x] Use directory handles in kernel options and retain report paths.

**Validation**

- [x] Real systemd mount probe.
- [x] Native drop-in order and read-only binds.
- [x] Literal source path characters.

## Milestone 2: Review and source resolution

**Work items**

- [x] Resolve typed and native binds through shared overlay source resolution.
- [x] Hold repository admission and use Start names throughout user app Review.
- [x] Check private-root working directories without implicit binds.
- [x] Accept missing-ok directories; skip required RootImage directory checks and warn by app and path.
- [x] Canonicalize Dora runtime paths before matching overlay trees.
- [x] Attach untracked-write notes to shared sources and incomplete coverage.

**Validation**

- [x] Working-directory rejection before mounts.
- [x] RootImage Review warning.
- [x] Symlinked Dora runtime redirection.
- [x] Native specifiers and coverage notes.

## Milestone 3: Cancellation and failure

**Work items**

- [x] Stop pending setup at checkpoints and drain in-flight operations before cleanup.
- [x] Reconcile partial overlays, unit links and drop-ins; accept repeated Cleanup and Reset.
- [x] Capture systemd startup reasons before teardown and preserve them in run results.
- [x] Observe late systemd setup failures through native state notifications.
- [x] Verify unit state and empty kernel service cgroups before collecting changes.

**Validation**

- [x] Stop during partial setup.
- [x] In-flight operation settles before cleanup.
- [x] Cleanup twice.
- [x] Late native working-directory failure under both write policies.
- [x] Busy overlay and lingering writers.
- [x] Report-save failure and controller restart.

## Milestone 4: Shared finalization and compatibility

**Work items**

- [x] Use neutral filesystem step IDs for both backends.
- [x] Check stored events, run records and consumers; retain opaque legacy ID strings.
- [x] Update the demo's generated filesystem step identity.
- [x] Save file changes before unmount and policy application; carry results to Console.
- [x] Leave shared write-policy adoption in #361 and check the stacked follow-ups.

**Validation**

- [x] Legacy event decoding and display.
- [x] Compose and systemd change-record equivalence.
- [x] Write-back, discard and direct run output.
- [x] Owned runtime resources removed.

Scripted regressions are TypeScript in [overlays.mts](../tests/overlays.mts).
Capability-required tests build unprivileged and elevate only the test executable.
