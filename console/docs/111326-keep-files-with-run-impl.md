# Keep files with the run: implementation plan

Created November 13, 2026. Status: implemented. The systemd overlay handoff and log-program
downloads await their sibling branches.
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

- [x] Add `Disposition::Keep` with a kept-folder destination inside the run folder.
- [x] Reuse write back's plan from `direct::inspect`; copy into `kept/<n>/` and record nothing on the host.
- [x] Verify every planned entry's type and regular-file and symlink sizes, sync, then remove the upper. Directory verification checks type; symlink size is its target length.
- [x] On failure keep the upper and report it; block verified cleanup.
- [x] Record the policy and each `n` to host folder mapping in the report.

**Validation**

- [x] Kept files match added and modified entries exactly.
- [x] Host folder byte-identical before and after.
- [x] Deletions reported, nothing removed.
- [x] Full disk and interrupted copy keep the upper.
- [x] Upper and run folder on different filesystems.
- [x] Output folder files stay put under all three policies.
- [x] Discard keeps the change list and no files.

## Milestone 2: Setting, Review and Result

**Work items**

- [x] Add `WritePolicy write_policy` to `ConsoleConfig` with values
  `WRITE_POLICY_UNSPECIFIED` (write back), `WRITE_POLICY_WRITE_BACK`,
  `WRITE_POLICY_KEEP` and `WRITE_POLICY_DISCARD`.
- [x] Pass it to Compose runs, and carry it in the common RunPlan for systemd overlay integration. Until then, systemd rejects keep/discard at Review.
- [x] Show the policy at Review and the policy and kept location with the result.

**Validation**

- [x] Projects with no setting behave as before.
- [x] A second keep run starts from the unchanged host folder and keeps its own files.
- [x] Desktop and narrow-screen rendering.

## Milestone 3: Download action

**Work items**

- [x] Add one `ArtifactKind` for run files. The request names a change-list position or a program.
- [x] Resolve change positions against the saved report and open beneath `kept/<n>/` or the output folder, without following symlinks. The program selector awaits the sibling's log index.
- [x] Send every file as an attachment with an encoded filename.
- [x] Show a Download action on kept and output folder entries in File changes.
- [ ] Attach log-dump resolution and actions when [journal export](../../libs/workload-lifecycle/docs/111026-run-logs.md#later-log-dump) lands; the sibling owns this integration.

**Validation**

- [x] Kept, non-UTF-8 and output folder files download intact.
- [ ] Log dump files download intact; requires the sibling integration.
- [x] Console records, other runs, traversal, hard-link aliases and symlink escapes refused.
- [x] A kept HTML file downloads and never renders.

## Tests

Capability-required tests build unprivileged and elevate only the test executable.
Scripted end-to-end checks are TypeScript.
