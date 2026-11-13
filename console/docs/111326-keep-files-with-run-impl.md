# Keep files with the run: implementation plan

Created November 13, 2026. Status: planned.
[Architecture](111326-keep-files-with-run.md). Terms follow the overlay design's
[term list](111026-file-change-report.md#terms).

## Starting point

| Piece | Where |
| --- | --- |
| Write policy | `host_filesystem::overlay::Disposition` (`WriteBack`, `Discard`) and `finalize` |
| Write-back plan | `overlay::direct::inspect`, then `rsync` with `changed.paths0` |
| Policy input | `OverlayPolicy` in `compose-apps` preparation; systemd gains it in the [systemd overlays plan](https://github.com/Cutuy/doramcap/pull/353) |
| Console setting | `ConsoleConfig` in `console-config.proto` |
| Download route | `artifacts.ts` and `service.ts` `openArtifact`, `ArtifactKind` in `console.proto` |

## Milestone 1: Keep policy in `host-filesystem`

**Work items**

- [ ] Add `Disposition::Keep` with a kept-folder destination inside the run folder.
- [ ] Reuse write back's plan from `direct::inspect`; copy into `kept/<n>/` and record nothing on the host.
- [ ] Verify type and size of every planned entry, sync, then remove the upper.
- [ ] On failure keep the upper and report it; block verified cleanup.
- [ ] Record the policy and each `n` to host folder mapping in the report.

**Validation**

- [ ] Kept files match added and modified entries exactly.
- [ ] Host folder byte-identical before and after.
- [ ] Deletions reported, nothing removed.
- [ ] Full disk and interrupted copy keep the upper.
- [ ] Upper and run folder on different filesystems.
- [ ] Output folder files stay put under all three policies.
- [ ] Discard keeps the change list and no files.

## Milestone 2: Setting, Review and Result

**Work items**

- [ ] Add `WritePolicy write_policy` to `ConsoleConfig` with values
  `WRITE_POLICY_UNSPECIFIED` (write back), `WRITE_POLICY_WRITE_BACK`,
  `WRITE_POLICY_KEEP` and `WRITE_POLICY_DISCARD`.
- [ ] Pass it to Compose runs, and to systemd runs once they have overlays.
- [ ] Show the policy at Review and the policy and kept location with the result.

**Validation**

- [ ] Projects with no setting behave as before.
- [ ] A second keep run starts from the unchanged host folder and keeps its own files.
- [ ] Desktop and narrow-screen rendering.

## Milestone 3: Download action

**Work items**

- [ ] Add one `ArtifactKind` for run files. The request names a change-list position or a program.
- [ ] Resolve it against the saved report; open beneath `kept/<n>/`, the output folder or the log folder, without following symlinks.
- [ ] Send every file as an attachment with an encoded filename.
- [ ] Show a Download action on kept and output folder entries in File changes, and on
  log dumps once [journal export](../../libs/workload-lifecycle/docs/111026-run-logs.md#later-log-dump) lands.

**Validation**

- [ ] Kept, non-UTF-8, output folder and log dump files download intact.
- [ ] Console records, other runs, traversal and symlink escapes refused.
- [ ] A kept HTML file downloads and never renders.

## Tests

Capability-required tests build unprivileged and elevate only the test executable.
Scripted end-to-end checks are TypeScript.
