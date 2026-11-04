![Machine capture and A/B comparison](110426-machine-capture-comparison.svg)

# Machine capture and A/B comparison

Created: 2026-11-04. Status: proposed.

Each run saves what it ran and the machine it ran on. Compare then shows what is the same and what differs across two past runs.

- **Today:** history keeps profiles, system apps, times, state and artifacts. Machine snapshots stay as raw JSON in private run files.
- **Next:** a typed capture, saved per run and shown at Review and Compare.
- **Later:** the same crate gains apply and verify for machine preparation.

Concept: [mockups](../mockups/README.md).

## Decisions

| Topic | Decision |
| --- | --- |
| Owner | New `libs/resources/system-config` crate. `observe()` ships first; apply and verify reuse the schema later. |
| Differences | Setup, configuration and condition are grouped. Only configuration differences warn. Nothing blocks Start or Compare. |
| Timing | Review shows an unsaved preview. Prepare saves the before state and Report saves the after state. |
| Values | Shown unless a name pattern masks them. Compose secrets are always hidden. |
| Baseline | Review uses the latest completed run with a capture. Run again uses its source run. |
| Fields | Performance focused, listed below. No new crates. |
| Availability | Every fact is captured or unavailable. Unavailable never compares as same. |
| Failure | An unreadable field never fails a run. |
| Storage | `console.db` holds facts and value hashes. Compare reads values from `runs/<id>/` and masks them at display time. |
| Condition | Both values are shown side by side, with no thresholds. |

## Captured facts

| Group | Owner | Facts |
| --- | --- | --- |
| Identity | Console | Run ID, container IDs, paths and times. Excluded from the diff. |
| Experiment setup | Console | Source commit, dirty tree, profiles, system apps, image IDs, environment values, config file hashes, Console version |
| Machine configuration | `system-config` | Machine fingerprint, architecture, CPU model and count, NUMA nodes, memory, kernel release, boot flags, isolated CPUs, governor, CPU frequency driver, turbo, SMT, huge pages, swappiness, RT limit, cgroup setup, GPU and driver |
| Docker configuration | `workload-docker` | Docker and Compose versions, cgroup driver, storage driver |
| Machine condition | `system-config` | Boot ID, uptime, load, available memory, free disk, temperature |
| Docker condition | `workload-docker` | Other running containers |

Per-CPU settings are stored by CPU range. The machine ID is stored only as a hash. Process and container facts keep names without command lines.

## Open items

| Item | Recommendation | Resolve in |
| --- | --- | --- |
| Initial mask patterns | Short list covering password, secret, token, key and credential names | Milestone 5 |
| User-defined masks | Add a `console.pbtxt` list only if the initial patterns prove insufficient | Milestone 5 |
| Boot flags display | Apply the masks to individual flags | Milestone 5 |
| GPU model source | Read `/proc/driver/nvidia/gpus/*/information`; confirm on a CUDA host | Milestone 1 |
| Evidence bundle redaction | Design with the bundle export; out of scope here | Bundle export plan |

Deferred: pinned baselines, busy-process sampling, condition thresholds and parsing values out of config files.

## Milestone 1: Crate and schema

**Work items**
- [ ] Add `libs/resources/system-config` with `MachineConfiguration` and `MachineCondition` protobuf messages, using `optional` fields for availability.
- [ ] Implement `observe()` with direct `/proc` and `/sys` reads. Report turbo from `intel_pstate/no_turbo` or `cpufreq/boost`.
- [ ] Hash the machine ID and group per-CPU settings by CPU range.
- [ ] Add a boundary rule keeping Docker and Console dependencies out of the crate.

**Validation**
- [ ] Fixture trees cover present, missing and malformed files for each fact.
- [ ] A missing file yields an absent field and never an error.
- [ ] Mixed per-CPU values produce separate CPU ranges.
- [ ] The raw machine ID never appears in the output.
- [ ] `observe()` runs on x86 and ARM64 without root.

## Milestone 2: Workflow capture

**Work items**
- [ ] Replace `machine-before.json` and `machine-after.json` with `system-config` capture in the Prepare and Report steps.
- [ ] Add Docker configuration and condition facts through the existing `bollard` client.
- [ ] Add the capture to `Inspection` for Review.
- [ ] Publish before and after states through `ExecutionUpdate` as each is captured.

**Validation**
- [ ] The before state is taken after reset and before any service starts.
- [ ] An unreadable field or unavailable Docker fact leaves the run running and the field absent.
- [ ] The after state still arrives after a failed run.
- [ ] Existing recorded workflow tests pass with the old snapshot files removed.

## Milestone 3: Run record

**Work items**
- [ ] Add `ExperimentSetup` and a run capture message to `console.proto`.
- [ ] Record source commit and dirty tree through the `git` command; non-git projects read as unavailable.
- [ ] Record image IDs, profiles, system apps, config file hashes, environment value hashes and Console version.
- [ ] Add an additive `console.db` migration with a size-bounded capture column.
- [ ] Save each state as it arrives.

**Validation**
- [ ] Migration step, failed-step rollback and reading rows without a capture.
- [ ] A failed run and a `kill -9` run both keep their before state.
- [ ] A capture over the size bound is rejected without failing the run.
- [ ] A dirty tree, a clean tree and a non-git project record correctly.
- [ ] Environment values never appear in `console.db`.

## Milestone 4: Review preview

**Work items**
- [ ] Add a TypeScript diff that walks the capture messages and reports same, changed or unavailable per field.
- [ ] Select the latest completed run with a capture as the baseline.
- [ ] Add a baseline run field to `PrepareRequest`, set by Run again.
- [ ] Add a Review panel with grouped differences and current condition values.

**Validation**
- [ ] The diff classifies same, changed, one-side-absent and both-absent fields.
- [ ] Baseline selection skips failed, interrupted and uncaptured runs.
- [ ] Run again compares with its source run.
- [ ] No baseline shows current values and an empty-baseline message.
- [ ] Configuration differences warn and Start stays available.
- [ ] A machine change between Review and Start does not cause a stale review.

## Milestone 5: Compare

**Work items**
- [ ] Add a compare route behind the session and origin checks, taking two run IDs.
- [ ] Read values from each run's private files, apply masks on the server and hide Compose secrets.
- [ ] Add `#/compare` with run pickers and groups for setup, configuration and condition.
- [ ] Show configuration or condition changes between a run's before and after states.
- [ ] Add a Compare action from History.
- [ ] Resolve the mask and boot flag open items with the user.

**Validation**
- [ ] 401 without a session, 403 for a foreign origin and 404 for unknown runs.
- [ ] Masked values and Compose secrets are absent from API responses as well as the screen.
- [ ] Hashes never reach the browser.
- [ ] Traversal or symlinks in run files return unavailable values.
- [ ] Runs from before this feature show unavailable facts.
- [ ] Loading, failed and missing-file states render, with no horizontal overflow at 390 px.

## Milestone 6: Real experiments

**Work items**
- [ ] Run real Compose A/B pairs through a real Console with a parameter change and a governor change.
- [ ] Run the capture on ARM64, WSL and a CUDA host, and on a Jetson when one is available.
- [ ] Capture screenshots of Review and Compare for this plan.

**Validation**
- [ ] The parameter change appears under setup with both values.
- [ ] The governor change appears as a configuration warning.
- [ ] A governor change during a run appears as a before and after difference.
- [ ] Facts missing on each platform show as unavailable.
- [ ] History and Compare survive `kill -9` and SIGTERM restarts.
- [ ] Full local CI and ARM64 validation pass at the PR integration point.
