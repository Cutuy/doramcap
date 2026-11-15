# File-change associations: implementation

Created November 15, 2026. [Architecture](111526-file-change-associations.md).

## Milestone 1: capture facts

**Work items**

- [x] Document User/System source usage in `AppScope`; reuse `SourceKind` on `FileChange`.
- [x] Carry scope in directory mounts and verified owned-container inputs.
- [x] Assign User/System roles in Compose and systemd, counting Dora as System.
- [x] Preserve protobuf values, earlier JSON names and unknown legacy defaults.

**Validation**

- [x] Persisted run-output associations
- [x] Stopped uppers and nested aliases
- [x] Compose role assignments
- [x] Docker layer associations
- [x] systemd mount associations
- [x] Legacy record defaults
- [x] Earlier JSON enum names

## Milestone 2: present and export

**Work items**

- [x] Label source handling and app association independently in Result and History.
- [x] Retain counts, order, download positions and complete JSON Lines export.
- [x] Keep the existing preview byte budget and incomplete labels.

**Validation**

- [x] User/System and legacy source labels
- [x] JSON Lines and preview compatibility
- [x] Browser rendering and downloads
- [x] Focused type and boundary checks
