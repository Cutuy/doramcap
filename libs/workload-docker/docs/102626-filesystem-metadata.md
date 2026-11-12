![Metadata collection](102626-filesystem-metadata.svg)

# Filesystem metadata collection

Created: 2026-10-26. Updated: 2026-11-12.
Scope: observe changes while preserving their storage category.

## Contract

`PrepareFilesystemSources` resolves mount identities and coverage without walking
host folders. `CollectFilesystemChanges` runs after all writers stop and saves
three manifests: host binds, container writable layers and named volumes. Rows
retain their source, consumers, change kind and available stat metadata. No file
contents are read or exported. Engine changes have no stat metadata.

Host changes come from stopped OverlayFS uppers, using the same interpreter as
write-back. A copied-up entry matching the lower's documented metadata is omitted.
Deleted folders expand into individual deleted paths. Fresh run output folders
stay directly mounted and list their files as added. Each record owner supplies
its private run-relative paths; a whole-run mount is reported as partly listed.

Host shared memory, including hugetlbfs and aliases of `/dev/shm` at other paths,
stays live and uses its existing shared-memory observation contract. It is excluded
from the file-change category without downgrading that category's coverage.
Single-file and special mounts, pseudo-filesystems, sources containing mountpoints
or the run folder, and Docker named volumes remain direct with coverage reasons.
Unsafe overlaps with an overlay fail before mounting. Supported writable and
read-only aliases share one view and retain their access flags.

Review and preparation share source classification and alias resolution. Missing
sources are explicitly unavailable at inspection time; Docker's permitted source
creation occurs during preparation. Disabling overlays reports ordinary writable
host folders as untracked in both Review and the result.

The report is synced and published before strict unmount, write-back and removal.
Expected changes and coverage gaps are informational. Required Engine observation
or artifact-write failure preserves evidence and blocks automatic removal.

## Resource usage and write-back

Preparation reads source and mount metadata. Collection walks the upper and any
lower subtrees hidden by deletions, allocating with their size. Output listing is
incremental, with open directory handles proportional to depth. There are no
entry, depth or time cutoffs. Docker's client materializes its Engine diff response.

Metadata comparison uses type, permissions, owner, modification time, symlink target
and non-directory size; it does not prove content equality. Shared sources are
deduplicated. Manifests stream length-delimited protobuf records; full downloads
remain available beyond the bounded Console preview.

Write-back replaces conflicting host edits on paths the run changed. It skips
sockets, FIFOs and devices and copies hard links as separate files, with warnings.
Basic rsync attributes are preserved; ACL, xattr, capability and hard-link topology
fidelity remain unqualified. Failed write-back retains the upper and diagnostics.

## Validation

Current work and checks are recorded in the
[file-change implementation plan](../../../console/docs/111026-file-change-report-impl.md#milestone-2-read-the-change-list-from-the-uppers).
The [2026-10-26 validation record](102626-filesystem-validation.json) is historical
evidence for the earlier before/after implementation; it does not qualify the
current upper-based collector.
