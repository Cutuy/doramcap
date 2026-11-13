![Keep files with the run](111326-keep-files-with-run.svg)

# Keep files with the run

Created November 13, 2026. Status: design for review; implementation not started.
[Implementation plan](111326-keep-files-with-run-impl.md) ·
[Overlays for writable mounts](111026-file-change-report.md), milestone 5.

Console gains the third write policy, keep with the run, and a download action for the
files a run holds in its run folder. The agreed behavior is in the overlay design's
[write policy](111026-file-change-report.md#write-policy) and
[report and downloads](111026-file-change-report.md#report-and-downloads) sections.
Terms follow its [term list](111026-file-change-report.md#terms).

## Behavior

- **Keep with the run:** after the change list is saved, Console copies each upper's
  added and modified entries into the run folder. The host folder stays untouched.
  Deletions appear in the change list only.
- **Setting:** one Console setting selects write back, keep with the run or discard.
  Write back stays the default. The setting is approved.
- **Review** shows the policy. **Result** shows the policy and the kept location.
- **Downloads:** each kept file, output folder file and log dump in the change list or
  log list gets a download action. There is no file browser.

## Kept folder

| Path | Holds |
| --- | --- |
| `runs/<run id>/kept/<n>/` | Entries from the upper of overlaid host folder `n`, at their paths inside that folder |
| `runs/<run id>/filesystem/report.pb` | The change list, with the policy and each host folder `n` maps to |

Numbering keeps host folder names, which may be any bytes, out of kept paths.
`kept/` is reserved in the run folder, like `filesystem/` and `overlays/`: the output list
leaves it out, and keep fails with the upper intact when an app has already created it.

## Copy

The copy reuses write back's plan: the same entries, the same skips and warnings for
sockets, FIFOs and device files, and hard links copied as separate files. Only the
destination differs. The upper and the run folder may sit on different filesystems.

Console removes an upper only after its copy is verified: every planned entry exists in
the kept folder with the planned type, regular-file and symlink sizes match, and the
folder is synced. Symlink size is its target's byte length; directory size is filesystem-specific.
A failed or
interrupted copy keeps the upper and blocks verified cleanup, as a write back stopped by
an I/O error does.

## Downloads

| Request names | Server opens, by raw name bytes |
| --- | --- |
| A kept entry in the saved change list | The entry inside `kept/<n>/` |
| An output folder entry in the saved change list | The entry inside its output folder |
| A program's log dump | Its file in the run's log folder |

The browser never sends a path. The server resolves the request against the saved
report, opens the file beneath its folder without following symlinks, and always sends
it as an attachment with an encoded filename. Console's own records are never served.

`GET /api/artifacts/<run id>/run-file.download?change=<position>` names a zero-based position
across the report's categories in saved order. The complete JSON Lines list includes
each position; the bounded preview keeps those positions when it interleaves categories.
`?program=<encoded ID>` reserves the same route for the journal-export integration.
Exactly one selector is required. Program downloads return Missing until that integration lands.

Each request reads the saved manifests through verified EOF before opening its file.
Memory holds the checkpoint and one change record. The opener checks regular-file type,
saved size and a single hard-link name, and refuses all symlink components.

## Owners

| Part | Owner |
| --- | --- |
| Keep policy, copy and verification | `host-filesystem` |
| Passing the policy to Compose and systemd runs | `compose-apps`, `workload-systemd` |
| Setting, Review and Result display, download route | Console |

## Decisions to make

| Decision | Options | Recommendation |
| --- | --- | --- |
| How a download names an entry | The entry's position in the saved change list, or its path bytes | Position. The saved report never changes, so the position is stable. |
| Copy verification | Type and size per entry, or also a content hash | Type and size; hashing reads every byte twice. |

## Limits

- Run folders grow; Console has no pruning yet.
- State an app carries between runs resets, because each run starts from the unchanged host folder.

Applicable rules: D-01 to D-03, D-06 to D-08, D-11 to D-13; I-03, I-04, I-06, I-07.
