![Journal export](111326-journal-export.svg)

# Journal export

Created November 13, 2026. Updated November 14, 2026. Status: implemented.
[Implementation plan](111326-journal-export-impl.md) ·
[Application logs for a run](111026-run-logs.md), the follow-up to
[Cutuy/doramcap#314](https://github.com/Cutuy/doramcap/pull/314).

After a run's programs stop, Console exports each program's entries still in the host
journal into one file per program in the run folder. Capture stays with the host
journal under its existing policy, as [option A](111126-run-log-capture-options.md)
selected. A saved file holds the entries still available; complete output stays unknown.

## Behavior

- **Which programs:** every unit a systemd run starts and every container a Compose run
  starts. Containers with project-managed logging are skipped and listed as such.
- **When:** after every program stops, before the run's cleanup finishes. Container
  entries stay queryable by full container ID after the container is removed.
- **Output:** one plain text file per program, one entry per line with its Unix timestamp in seconds and microseconds. Embedded line breaks are escaped.
- **Result:** a Logs list naming each program with exported, skipped or failed, and the
  file size. Downloads use the shared [run-file route](https://github.com/Cutuy/doramcap/pull/358) with a saved log-index position.
- **Failure:** an export failure never changes the run's outcome. Files already written stay.

## Matching a program's entries

| Program | Identity recorded at start | Journal match |
| --- | --- | --- |
| systemd unit | Invocation ID, read from the service manager after start | `_SYSTEMD_INVOCATION_ID` for its own output, plus the service manager's entries carrying the same invocation |
| Container | Full container ID | `CONTAINER_ID_FULL` |

Unit names repeat across runs; invocation IDs and container IDs do not, so adjacent runs
never read each other's output.

## Run folder

| Path | Holds |
| --- | --- |
| `runs/<run id>/logs/<n>.log` | Program `n`'s entries |
| `runs/<run id>/logs/index.pb` | Each `n` with its program ID, identity, state, entry count and size |

Numbered storage files follow index positions. Download names use the saved program ID
with path separators, filename-reserved characters and controls replaced by hyphens;
an empty ID uses its position. Duplicate names retain separate download links.
The log folder is private, excluded from the change list and follows run-folder retention.
The index starts with pending identities and is replaced atomically after each export.
Export streams one entry at a time without a file-size ceiling. Console fits log metadata
within the existing encoded History and response budgets. Oversize is explicit; saved
files remain available on the host.

## Owners

| Part | Owner |
| --- | --- |
| Program list and identities, export order | Each workflow: `compose-apps`, `workload-systemd`, later `workload-holoscan` |
| Export step, files and index | `workload-lifecycle` |
| Capture, rate limits, retention | Host journal |
| Logs list and downloads | Console |

## Failure and recovery

| Event | Result |
| --- | --- |
| Journal unavailable | Every program reads failed; the run outcome is unchanged. |
| Export stops part way | Written files stay; the program reads failed with the reason. |
| Run storage full | Same as a stopped export; cleanup still runs. |
| Program exited almost at once | Its last lines may lack an identity and be missing. |
| Console stops before export | Export during the next admission's reset from saved identities. |

## Owner-approved decisions

Approved November 14, 2026.

| Decision | Behavior |
| --- | --- |
| Persisted record | One atomic `logs/index.pb` holds pending identities and export results. |
| Journal reader | Use `journalctl` with explicit field matches. |
| Text format | Unix seconds and microseconds; escape backslashes and line breaks. |
| Interrupted file | Preserve bytes, mark Failed, and recount complete saved entries. |
| Compose reset | Read full IDs and the export/skip choice from repository-owned containers' labels. |
| systemd reset | Before a new run, stop prior owned units, export their logs and remove resources; failed reset blocks admission. |
| systemd locator | A private per-repository `journal-run` symlink locates the prior run directory and its index. |
| Invocation fields | Match all four native invocation fields against the saved ID. |
| Console source | Read the current index for Result and History; retain recorded execution outcomes. |
| Display budget | Use actual encoded sizes against existing History and response budgets. |
| File-change exclusion | Shared lifecycle preparation excludes the run-relative `logs` subtree. |
| Identity writes | Serialized startup and cleanup/reset boundaries write identities; live inspection remains read-only. |
| Reset state | Clear blocked after verified reset, before new preparation. |
| Regression coverage | Extend the existing native abandoned-unit test with preparation rejection after reset. |
| Label contract | Generated protobuf metadata maps to the existing flat Docker label keys and values. |

### Docker labels

[`ResourceLabels`](../../docker-engine/proto/docker.proto) defines the six
`io.doramcap.*` fields. The [Docker adapter](../../docker-engine/src/labels.rs)
encodes the existing strings and retains absent fields for older resources. The
experiment-owner marker keeps its presence semantics; boolean policy labels enable
only the literal `true`. Ownership filtering continues to use the exact controller label.

The owner replayed and approved these six choices for PR #360 on November 13, 2026:

| Decision | Approved choice |
| --- | --- |
| Download selection | Select the saved log-index position; duplicate program IDs remain independently downloadable. |
| Partial downloads | Download exported files and nonempty failed files; label failed files as partial logs and keep their Failed state. |
| Download name | Derive a readable attachment name from the saved program ID; keep numbered storage files. |
| File authorization | Reuse the run-file route and confined opener; require a regular file, one hard link and the saved byte count. |
| Reset state | The runner clears blocked after verified reset; later preparation rejection preserves that cleanup result. |
| Regression coverage | Extend the existing native admission-reset test with rejected preparation and observable cleanup assertions. |

Applicable rules: D-01 to D-03, D-06 to D-08, D-10, D-12, D-13; I-03, I-05, I-06, I-07.
