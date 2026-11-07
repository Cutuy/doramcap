![Run request fields: end state](110726-run-request-fields.svg)

# Run request fields

Created: 2026-11-07. Status: agreed end state; deferred until the launch-forms stack lands.

Console sends the native addon one `RunRequest`: the shared `RunPlan` plus one backend part in a
`oneof`. Today start order, MCAP writers and Dora settings are defined inside each backend part.
They are backend-agnostic, so they move into `RunPlan`, leaving backend parts with only how each
app launches. Related plan: [launch forms](110726-launch-forms.md).

## Agreed end state

| Concept | Shared `RunPlan` | Compose part | systemd part |
| --- | --- | --- | --- |
| Run identity and budgets | id, repository, artifacts, recordings, readiness, stop | — | — |
| Start order | Groups of app IDs, each with its gate (wait for recordings) | — | — |
| Apps | — | One `ComposeProject` per app, keyed by app ID, in a plain list | Programs; adopted unit files whose ID is the file stem |
| MCAP writers | App IDs | — | — |
| Dora | Graph, coordinator and daemon ports, runtime directory, memlock, participant | Image | Executable |
| Host shared memory | — | — | `host_shm`; moves to `RunPlan` once Compose supports it |

The addon matches apps across parts by app ID and maps the request into the existing
`ComposeRun` and `SystemdRunner` inputs, so neither workflow's internals change.

## Milestone 1: Shared run request fields

**Work items**
- [ ] Add groups, MCAP writer apps and shared Dora settings to `RunPlan`.
- [ ] Reduce `ComposePlan` to a plain project list keyed by app ID plus the Dora image; the addon derives Compose's system/user split and MCAP service names from `RunPlan`.
- [ ] Reduce `SystemdPlan` to programs, adopted unit files, the Dora executable and `host_shm`.
- [ ] Console builds the shared fields once for both backends.

**Validation**
- [ ] Mapping tests: a request yields today's `ComposeRun` and systemd inputs for the release apps.
- [ ] Docker end-to-end suite; real Compose and systemd Console tests; native systemd tests.

## Notes

- Compose MCAP settings apply to every service of a writer app's project. That matches today's
  behavior for all release apps, which each have one service.
- `RunRequest` is not persisted, so the change needs no wire compatibility.
