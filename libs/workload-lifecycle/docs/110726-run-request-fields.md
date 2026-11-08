![Run request fields](110726-run-request-fields.svg)

# Run request fields

Created: 2026-11-07. Status: shared contracts landed; Console systemd execution remains pending.
Checked against remote main `65fe1432`.

Launcher-neutral run facts belong in `RunPlan`; backend plans describe app launch.
Rust enforces Dora → system apps and recording readiness → user apps, with reverse shutdown.

| Owner | Current contract |
| --- | --- |
| `RunPlan` | Identity, paths, recordings, budgets, `dora_apps`, `system_apps`, `mcap_apps`, `dataflow_file` |
| Preparation | Fills `RunPlan.dora_runtime` from app inputs; Compose consumes it today |
| `ComposePlan` | Projects keyed by app ID |
| `SystemdPlan` | Programs, app units with binds, repository unit files and binaries directory |
| App configuration | Common process/file settings in app pbtxt; native Compose files and systemd units retain backend settings |

Dora coordinator and daemon are ordinary apps selected by ID. Native launch files remain
native; Rust owns readiness, graph registration and control commands. Configurable startup
groups and backend-specific Dora settings are removed.

## Milestone 1: Shared run fields

Evidence: [#211](https://github.com/Cutuy/doramcap/pull/211) landed shared roles and app
configuration; [#214](https://github.com/Cutuy/doramcap/pull/214) added prepared Dora facts.
[#215](https://github.com/Cutuy/doramcap/pull/215) and
[#216](https://github.com/Cutuy/doramcap/pull/216) added systemd input capture and workflow steps.
See the [implementation record](110726-shared-run-fields.md) for validation evidence.

**Work items**
- [x] Share role IDs, recording writers and the dataflow artifact.
- [x] Flatten backend plans and enforce fixed startup phases in Rust.
- [x] Build shared Console fields once and preserve native launch inputs.
- [ ] Complete Console systemd execution and shared-runtime adoption ([#204](https://github.com/Cutuy/doramcap/pull/204)).

**Validation**
- [x] Shared-field mapping and Console contract tests.
- [x] Native Compose and systemd Dora startup, recording and shutdown tests.
- [ ] Real Console systemd execution end to end (#204).

Related: [launch forms](110726-launch-forms.md). New launcher-neutral fields belong in `RunPlan`.
