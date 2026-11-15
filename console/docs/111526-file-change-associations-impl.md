# File-change associations: implementation

Created November 15, 2026. [Architecture](111526-file-change-associations.md).

## Milestone 1: capture facts

**Work items**

- [x] Add prefixed `AppScope` and reuse `SourceKind` on `FileChange`.
- [x] Carry scope in directory mounts and verified owned-container inputs.
- [x] Assign roles in Compose and systemd; combine shared source associations.
- [x] Preserve unknown defaults for older protobuf and JSON records.

**Validation**

- [x] Persisted run-output associations
- [x] Stopped uppers and nested aliases
- [x] Compose role assignments
- [x] Docker layer associations
- [x] systemd mount associations
- [x] Legacy record defaults

## Milestone 2: present and export

**Work items**

- [x] Label source handling and app association independently in Result and History.
- [x] Retain counts, order, download positions and complete JSON Lines export.
- [x] Keep the existing preview byte budget and incomplete labels.

**Validation**

- [x] Shared and legacy source labels
- [x] JSON Lines and preview compatibility
- [x] Browser rendering and downloads
- [x] Focused type and boundary checks
