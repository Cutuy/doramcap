![One Console per project](110226-per-project-console.svg)

# One Console per project

Created: 2026-11-02.

Each project runs its own Console service with its own database and recordings, so
projects stay independent and can run at the same time.

| Topic | Decision |
| --- | --- |
| Start | `doramcap-console start` uses the current directory; `--project PATH` names another. |
| Project ID | Sanitized folder name plus the first 12 hex of the canonical path's SHA-256, such as `robot-3f9a1c2e7b04`. |
| Service | `doramcap-console-<id>.service`, transient through `systemd-run`. |
| State | `/var/lib/doramcap-console/projects/<id>/`: `console.db`, `console.pbtxt`, `project.txt`, `access.txt` and `runs/<run id>/`. Outside the repository. |
| Moved repository | A new path is a new project; old state stays behind. |
| Sample | `playground/cpu-burn` stays read-only inside the release. |
| Upgrades | Each `console.pbtxt` stays pinned to the release that wrote it. |

## Milestone 1: Free ports per Console

**Work items**
- [x] Select a free live bridge port at startup when `port` is omitted.
- [x] Omit the port from the release template and examples.

**Validation**
- [x] Config and port tests pass.

Evidence: Console server suite passes. Real concurrent runs are recorded under Milestone 2.

## Milestone 2: Per-project service and state

**Work items**
- [ ] Add `--project`, defaulting to the current directory.
- [ ] Derive the project ID, service name and state directory.
- [ ] Add `list`.
- [ ] Stop seeding a playground; keep the sample read-only in the release.

**Validation**
- [ ] Project ID and release tests pass.
- [ ] Two projects run at the same time with separate history and recordings.

## Milestone 3: One project in the UI

**Work items**
- [ ] Remove the project directory input and History's project filter.

**Validation**
- [ ] Frontend tests and a browser run pass.
