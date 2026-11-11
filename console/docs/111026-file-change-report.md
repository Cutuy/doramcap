# File changes in Console

Created November 10, 2026. Tracking note. Design has not started.

Docker-based runs record which files changed during a run. Console's web app
does not show that record yet. This note tracks the gap.

## Today

| Stage | What exists |
| --- | --- |
| Capture | `docker-engine` records a file baseline before launch and collects changes after writers stop. |
| Record | Each change has a storage category, source, path, kind (added, modified, deleted) and file metadata before and after. File contents are never read. |
| Storage | One manifest file per storage category in the run's private artifacts folder, with counts, a coverage flag and warnings. |
| Compose result | The Compose run report carries the counts, coverage and manifest paths. |
| Console backend | The execution result sent to the server has no file-change fields. |
| Web app | Shows the baseline capture as a progress step. Shows no changes. |
| Run report download | `console-report.json` has no file-change section. |

Storage categories are host folder mounts, container writable layers and named volumes.

## Gap

A user cannot see from Console which files a run added, changed or deleted, or
whether capture covered every mount.

## Workflows affected

| Workflow | File capture |
| --- | --- |
| Compose | Yes |
| Holoscan (planned) | Yes. The file list is its only record of what the app wrote, since Console runs no recording check for it. |
| systemd | None today |

## Open questions

- What the web app shows: counts and coverage only, or a browsable file list.
- How a long file list reaches the browser: in the execution result, or as a downloadable artifact.
- Where mounts outside capture coverage appear: Review, the run result, or both.
- Whether history and run comparison include file-change counts.
- Whether systemd runs get file capture.
