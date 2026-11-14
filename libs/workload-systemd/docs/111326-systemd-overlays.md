![systemd overlays](111326-systemd-overlays.svg)

# systemd overlays

Created November 13, 2026. Decisions replayed November 14, 2026.
[Implementation plan](111326-systemd-overlays-impl.md) ·
[Overlays for writable mounts](../../../console/docs/111026-file-change-report.md), milestone 4.

A systemd run uses shared overlays, file changes and finalization. Terms follow the
[overlay term list](../../../console/docs/111026-file-change-report.md#terms).

## Behavior

Each declared writable host directory gets one overlay before units start. Units
keep their destinations and access modes. After every writer stops, the run saves
file changes, unmounts and applies the selected write policy. Console shows the
shared File changes report, including incomplete coverage and untracked writes.

| App | Launch inputs | Overlay delivery |
| --- | --- | --- |
| System app | Process settings generate a transient unit and private root | Generated bind properties |
| User app | Captured unit files and native drop-ins | One owned drop-in rebuilds both bind lists |

Review holds repository admission while inspecting user apps under the exact Start
names and removing its temporary links. systemd resolves native settings and
specifiers; Console adds no unit-file parser.

## Working directory

Review and Start check working directories before mounting. System apps must use
paths visible in their private root; no working-directory bind is added implicitly.
`WorkingDirectory=-/path` accepts absence. For a required directory inside
`RootImage=`, Review names the app and unchecked directory; systemd validates it at
Start. A user app's working directory outside writable mounts is listed as untracked.

## One run

1. **Review:** resolve mounts and working directories; list coverage limitations.
2. **Setup:** link captured units, resolve native settings and mount overlays.
3. **Launch:** start Dora, system apps and user apps with redirected sources.
4. **Stop:** verify unit states and empty kernel service cgroups.
5. **Report:** read uppers and durably save file changes.
6. **Finalize:** strictly unmount, apply the write policy and remove owned resources.

Stop cancels pending setup. An attempted side effect finishes before cleanup;
checkpoints prevent further setup work. Partial resources belong to the scope from
the first attempt. Cleanup and Reset accept missing resources and can be repeated.

## Approved decisions

| # | Decision | Behavior |
| --- | --- | --- |
| 1 | Shared step IDs | Pure rename to `workload_lifecycle::filesystem::…`; stored IDs remain readable as opaque strings. |
| 2 | User app mounts | One owned drop-in resets and rebuilds both native bind lists, preserving destinations, modes, flags and order. |
| 3 | User app Review | Use Start names; hold exclusive repository admission through inspection and cleanup. |
| 4 | Write policy | Filesystem owns the shared policy carried by `RunPlan.write_policy`; omission preserves write-back. #361 replaces #357's interim `SystemdPlan.discard_writes`. |
| 5 | Setup cancellation | Stop cancels pending setup; cleanup waits for attempted operations and reconciles partial resources. |
| 6 | Host paths | Open directory handles supply kernel mount options; reports retain host paths. |
| 7 | Stopped writers | Verify service state and kernel cgroup population before collection; preserve resources if unverified. |
| 8 | RootImage directory | Skip the host check, warn at Review and let systemd validate at Start. Preserve its failure reason. |
| 9 | Untracked writes | Warn at Review and add coverage notes to the shared File changes report; mark coverage incomplete. |

## Owners

| Part | Owner |
| --- | --- |
| Units, drop-ins, directory checks, setup cancellation | `workload-systemd` |
| Overlays, file changes, write policy | `host-filesystem` |
| Run order, admission, shared steps | `workload-lifecycle` |
| Report, preview, downloads | Console |

## Failure and recovery

| Event | Result |
| --- | --- |
| Checkable working directory missing | Review or setup fails with the app and path before mounts or startup. |
| Setup fails or Stop arrives | No further setup or launch; drain attempted operations and reconcile partial overlays, links and drop-ins. |
| User app fails at Start | Run fails with systemd's reason; stop all units, save changes and apply the selected write policy. |
| Shutdown or report publication unverified | Preserve resources and block verified cleanup. |
| Busy overlay | Preserve the workspace; Cleanup retries strict unmount after the holder exits. |
| Controller crashes | Next admission stops owned units and discards abandoned overlays. |

## Limits

The service manager must see the controller's mounts. Writes outside declared
writable mounts remain untracked. RootImage contents are checked by systemd at
Start. Run output directories retain their shared direct-output semantics.

Applicable rules: D-01 to D-04, D-06 to D-10, D-12, D-13; I-03 to I-07, I-09.
