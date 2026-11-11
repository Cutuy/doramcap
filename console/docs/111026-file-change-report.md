![Shared filesystem capture](111026-file-change-report.svg)

# File changes in Console

Created November 10, 2026. Updated November 11, 2026. Architecture agreed; implementation pending.
[Implementation plan](111026-file-change-report-impl.md).

Docker runs already create private filesystem views, collect file changes and finalize
writes. Console drops that report. Systemd has no equivalent tracked-directory workflow.
Reuse the existing filesystem mechanism for both and expose one report in Console.

## Agreed behavior

- Capture supported resolved writable directory binds and declared output directories.
- Show counts, coverage and a bounded file preview on current and past run results.
- Download the complete collected list as JSON Lines.
- Keep host `/dev/shm` observation separate. Preserve native device and shared-memory access.
- Keep launch fingerprints and machine observations in their existing roles.

This extends the [Compose/Holoscan capture design][outputs]. Holoscan keeps its own
CLI launch integration. This change does not implement the Holoscan runner.

## Shared mechanism

| Owner | Responsibility |
| --- | --- |
| `host-filesystem` | Existing Linux directory views, metadata collection, manifest records, upper-layer interpretation, write-back and discard. |
| `workload-lifecycle` | Shared capture steps and ordering, from prepared storage through saved file-change report and finalization. |
| Docker backends / `docker-engine` | Resolve binds and volumes, wire views into native launch, verify Docker visibility, collect Engine layer/volume observations and own Docker resources. |
| systemd backend | Resolve effective directory binds, wire views through systemd, stop units and verify their mount references are released. |
| Console | Persist the report reference and summary, build a preview and stream the complete download. |

Extract the existing Linux implementation from `docker-engine`; carry its tests and
persisted formats. Use the current crates and dependencies. Keep Docker Engine calls
and service-manager calls with their backends. The shared run sequence calls the same
capture/finalization operations for each backend; it does not infer writes from launch
fingerprints or machine-state comparisons.

## Run sequence

1. **Review:** show capture sources and known exclusions from the resolved launch.
2. **Start preparation:** revalidate the launch, prepare sources and shared views, and
   save the file baseline before any application writer starts.
3. **Launch:** each backend maps the returned views at the original application targets.
4. **Stop:** stop every attempted writer, including after partial startup or cancellation.
5. **Collect:** read tracked views and backend observations; save the manifests and
   file-change report while the views and stopped Docker containers remain available.
6. **Finalize:** apply the existing write-back/discard policy, release view references,
   unmount views, and remove owned runtime resources after required collection succeeds.

The report records observation, write disposition and cleanup separately. Collection
must precede destructive storage finalization. Systemd unit mount references must be
released before strict unmount; a busy view remains owned and blocks verified cleanup.
Machine-observation timing remains unchanged.

## Sources and native behavior

| Source | Behavior |
| --- | --- |
| Supported persistent directory binds | Reuse shared OverlayFS views; preserve destinations, shared aliases and access flags. Systemd uses the existing write-back default, with no new setting. |
| Fresh run-owned output directories | Retain their existing private paths; collect through the common file mechanism. Exclude controller-owned records. A shared run root without separable application output has explicit partial coverage. |
| Docker writable layers / named volumes | Preserve existing Engine diff, metadata and volume-retention behavior. |
| Read-only aliases into a tracked tree | Preserve the alias relationship and read-only access using the same view mapping. |
| Devices, live shared memory and unsupported private storage | Preserve native access and list the exclusion. No filesystem-content capture claim. |
| Arbitrary native-unit writes outside resolved sources | Outside capture coverage; retain native unit semantics. |

Systemd reads effective bind properties through the service manager and retains native
optional-source, target, ordering and access semantics. Review identifies sources that
cannot be resolved until runtime preparation as unknown. Start resolves them before
writers start and records the actual inventory. An application-visible path is never
silently redirected to its original source after planned view creation fails.

Keep current restrictions on overlay backing, overlapping roots, nested mounts and
hardlinks. Validate capture constraints before launch. Do not overlay the entire host
filesystem or add user-selected roots or environment inputs.

## Failure and recovery

| Event | Result |
| --- | --- |
| Expected file writes | Informational counts and records. |
| Excluded or incompletely scanned source | Explicit scope/reason; observed counts cover successful sources only. Infer no deletions from a failed scan. Preserve existing named-volume retention rules. |
| Planned view setup fails | Do not launch writers; clean up only newly owned views, retaining failed cleanup state. |
| Writers remain active | Do not claim a final file list or remove their storage. |
| Required collection or saving the file-change report fails | Retain runtime resources needed for recovery, report the failure and block verified cleanup. Apply this existing Docker preservation rule to systemd capture. |
| Write-back fails or original files changed externally | Preserve remaining upper data and diagnostics. Do not replay writes automatically. |
| Controller stops unexpectedly | The next admission stops owned writers before the existing reset/discard path. Salvage is best effort; an interrupted report remains unavailable or explicitly incomplete. |

A saved file-change report means change manifests, counts and coverage were recorded.
It contains metadata; native recording validity has its own outcome. Final write-back
is non-atomic and keeps its existing metadata-fidelity limitations.

## Console and persistence

Run-private manifests are the authoritative file list. A completed checkpoint names
only synced manifests inside that run and is published atomically. Console saves its reference and summary with the run
transaction, and includes the section in `console-report.json`. The preview is derived
from those records; refresh, History and restart use the same report.

Show storage category, source, consumers, path, change kind and available before/after
metadata. Counts describe metadata transitions, not distinct file contents. Display
coverage and download availability independently of execution and recording outcomes.
Old runs without a report read as unavailable. A complete zero-change observation
reads as zero; a missing report never does.

Keep all records on disk. Bound the preview to the existing transport and saved-detail
budgets, label it as a preview, and retain access to every collected record through the
authenticated artifact route. Preserve raw Unix path bytes in the download and show a
readable escaped form in the browser. Stream JSON Lines with backpressure and explicit
export completion; malformed or missing manifests cannot produce a complete export.

Use the current run retention and private-file permissions. Do not add a filesystem
search API, pagination, run-comparison fields or new history-list columns in this work.
The original tracking PR is [#312](https://github.com/Cutuy/doramcap/pull/312).

## Qualification

This is a design, based on source inspection. Shared Linux capture exists for Docker;
its extraction, systemd mount visibility and end-to-end Console delivery remain untested.
The implementation plan covers real Compose/systemd runs, failure preservation,
restart, full download readback and ARM64. GPU and Holoscan qualification remain separate.

Applicable rules: D-01 through D-14; I-03 through I-07 and I-09.

[outputs]: https://github.com/Cutuy/doramcap/blob/90fb3d1766ffbf4f4e6eb66d0f90cb0c999a5933/libs/workload-holoscan/docs/110926-holoscan-outputs.md
