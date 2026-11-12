![Run directory layers](111026-file-change-report.svg)

# Run directory layers

Created November 10, 2026. Updated November 11, 2026: reworked around overlay layers.
Status: architecture proposed; milestone 1 implemented in review, the rest pending.
[Implementation plan](111026-file-change-report-impl.md).

Every run gets its writable directories the same way, for Compose and systemd: one
overlay per writable mount. The overlays' upper layers hold everything the run wrote.
Console reads them for the file-change list, applies the write policy to them and
reports both.

## Goal

1. **A launch sees only the mounts it declares.** Compose services, services Console
   creates and your own unit files all declare their mounts in their own format.
2. **Every writable mount is an overlay.** The host folder is the lower layer and a
   private run layer is the upper. A fresh run output folder has an empty lower layer.
3. **The changed files are the upper layers.** After writers stop, Console walks each
   upper layer. The lower layer stays unchanged during the run, so it supplies the
   "before" metadata.
4. **The write policy acts on the upper layers:** write back, keep with the run, or discard.
5. **The working directory is a path in the app's view.** It reaches a host folder
   only through a declared mount, as in Compose.

Machine snapshots (three per run) and input copies (frozen launch inputs) keep their
current owners and are outside this design.

## Layers

| Layer | Does | Owner |
| --- | --- | --- |
| Host folders | Hold the data the app reads and, after write back, its results. | The host |
| Declared mounts | List each mount: host source or none, app path, read-only or writable. | Backend |
| Overlay per writable mount | Mount lower + upper; the merged view is what the app gets. Read-only mounts bind directly. | `host-filesystem`, ordered by `workload-lifecycle` |
| App view | Map each merged view at its app path: a Compose bind or a systemd `BindPaths=`. | Backend |
| Change list | Walk upper layers into added, modified and deleted records. | `host-filesystem` |
| Write policy | Apply, keep or drop the upper layers. | `host-filesystem` |
| Report and downloads | Save the report, show a preview, stream the list, serve kept files. | Console |

Backends own only their native steps: declaring mounts, mapping views, launching and
stopping writers. Everything that touches overlays is shared.

## What each launch declares

| Launch | Writable mounts | Not tracked |
| --- | --- | --- |
| Compose service | Host folder binds; local named volumes (pending decision) | The container layer, read through Docker's diff as today |
| Service Console creates | Its declared binds and output folders | Nothing else is writable in its own root |
| Your own unit file | Its declared binds | Writes elsewhere on the host; Review lists the unit as partly tracked |

Console no longer binds a service's working directory automatically. A Console system
app keeps its working directory under a declared mount; tests check the Dora coordinator
and daemon (`/runtime`). A working directory with no mount under it fails to start with
systemd's own error.

Devices and `/dev/shm` stay direct in every launch.

## Write policy

| Policy | Upper layer goes to | Source folder | Run keeps |
| --- | --- | --- | --- |
| Write back (default) | The source folder | Modified | The list |
| Keep with the run | A folder inside the run | Untouched | The list and the files |
| Discard | Nowhere | Untouched | The list |

One Console setting selects the policy for the project. Write back stays the default,
so existing projects behave as before. This setting is approved.

- With keep, the next run starts from the unchanged source; state an app carries between
  runs resets.
- Run folders grow, and Console has no pruning yet. That is separate work.

## One run

1. **Mount overlays** for every writable mount before any writer starts.
2. **Launch** with each merged view at its app path.
3. **Stop** every attempted writer, including after partial startup or cancellation.
4. **Read upper layers** into the change list while the views stay mounted.
5. **Save the report**: sync the list, then publish its checkpoint atomically.
6. **Apply the write policy** to each upper layer.
7. **Unmount and clean up** owned views after the report is saved.

The change list must be saved before the write policy runs. A busy view stays owned and
blocks verified cleanup.

## Change list

- **Added:** in the upper layer, absent from the lower.
- **Modified:** in both; "before" from the lower, "after" from the upper.
- **Deleted:** a whiteout or an opaque directory in the upper layer.

Opening a file for writing or changing its metadata copies it into the upper layer even
when its contents are unchanged. Whether such files read as modified is a pending decision.

The list records metadata and paths with their raw Unix bytes. It does not record a
history of writes, only the final state of each upper layer.

## Report and downloads

Console saves the report reference and an 8 KiB preview with the run, shows them on the
Result and History pages, and streams the full list as JSON Lines. Old runs without a
report read as unavailable; a complete zero-change run reads as zero.

The run folder, `runs/<run id>/`, is the one place Console owns. It holds Console's
records, recordings, program log dumps
([Cutuy/doramcap#314](https://github.com/Cutuy/doramcap/pull/314)) and, with keep with the
run, the kept files.

Each kept file and log dump has a download action through the authenticated artifact
route. The browser names a kept file by its entry in the saved change list and a log dump
by its program; it never sends a path. The server opens the file only inside the run's
kept-files or log folder, by its raw name bytes. Files are always sent as downloads.

## Failure and recovery

| Event | Result |
| --- | --- |
| Overlay setup fails | No writer starts; only newly owned views are cleaned up. |
| Writers remain active | No final list; their views stay mounted. |
| Reading or saving the list fails | Views stay mounted for recovery; the failure is reported and blocks verified cleanup. |
| Write back fails | The remaining upper data is kept with diagnostics. Writes are never replayed automatically. |
| Console stops unexpectedly | The next admission stops owned writers, then discards leftover views as today. |

## Limits

- Overlay backing, overlapping writable mounts, nested mounts and hard links keep their
  current restrictions, checked before launch.
- The whole host filesystem is never overlaid.
- A file's copy into the upper layer breaks its hard links for the run.

## Pending decisions

| Decision | Options |
| --- | --- |
| Local named volumes | Overlay them like binds, or list them as not tracked. |
| Files copied up without content changes | Report them as modified, or compare with the lower file and drop identical ones. |

## Qualification

This is a design. Milestone 1 (the code move) is implemented and in review. Upper-layer
listing, systemd overlays and keep with the run remain unbuilt; the implementation plan
lists their checks.

Applicable rules: D-01 through D-04, D-06 through D-09, D-11 through D-13; I-03, I-04,
I-06 and I-07.
