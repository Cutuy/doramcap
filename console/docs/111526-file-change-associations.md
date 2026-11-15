![File-change associations](111526-file-change-associations.svg)

# File-change associations

Created November 15, 2026. [Implementation](111526-file-change-associations-impl.md).
Addresses finding 6 of the [rc7 report](111426-rc7-ux-report.md).

File changes retain two facts: which user/system apps mount their source and
how that source is observed. Console labels both beside each source on Result and
History. Every captured change remains available in the JSON Lines download.

## Behavior

| Fact | Values |
| --- | --- |
| Source consumers | User, System, User and System, unknown |
| Source handling | Existing `SourceKind`, including run output and overlay |

A source is a host folder, named volume or container layer. Its consumers are apps
that use it, including read-only mounts. Dora apps count as system apps. A source
mounted by both groups shows User and System; any unknown consumer role keeps the
association unknown. File writers are unknown. Run output can belong to either group.

Launch backends assign associations from resolved service roles before formatting
consumer IDs. Host-filesystem combines source associations and supplies source
handling. Docker preserves the backend's association on its layer observations.

## Persistence and scope

The existing `FileChange` stores both facts as additive protobuf fields. Published
manifests own the saved facts across navigation, restart and configuration changes.
Records without these fields keep unknown values. Renamed enum values retain their
protobuf numbers; JSON checkpoints accept the previous names. Consumer names and
paths do not determine app roles.

Storage categories, counts, coverage, record order, download positions and private
record exclusions keep their existing meaning. The bounded preview includes the new
fields in its existing byte budget; its incomplete label and full download remain.
Source headers group only records with the same source, facts and consumer list.

No dependency, environment input, metadata index or endpoint is added.

Applicable rules: D-03, D-07, D-08, D-13, D-15; I-03, I-06, I-07, I-09.
