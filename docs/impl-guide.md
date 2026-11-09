# Implementation guide

Code and test contracts, with their stated conditions and exceptions. Use alongside
the [design guide](design-guide.md); agent workflow, approvals, and validation
procedures remain in [AGENTS.md](../AGENTS.md).

## I-03 — Contracts and changes

- Dependencies follow established ownership decisions; use [repository boundary checks](../scripts/ci/boundaries.json). Inspect semantic ownership separately.
- Ownership changes update placement, public names, dependencies, callers, documentation, and relevant tests together.
- Refactors preserve runtime behavior and persisted/wire compatibility unless a change is requested. Assess actual consumers before removing public APIs; aliases must not preserve forbidden boundaries.

## I-04 — Inputs and implementation

- Validate configuration constraints during loading or preparation when possible, before dependent side effects. Check rendered-input constraints once rendering is available; errors identify the constraint and relevant input.
- Avoid hardcoded constants. Give required fixed values descriptive names and reuse them.
- Keep Python out of the application serving stack. It is allowed for tests, benchmarks, and development tooling.
- Encapsulate approved environment configuration behind a named interface listing supported variables; the Dora bridge uses `dora_app::environment`. Approval requirements are in [AGENTS.md](../AGENTS.md#configuration-approval).

## I-05 — Concurrency

- Outside tests, signal work arrival, capacity, state changes, completion, failure, and shutdown explicitly. Block on channels, `select!`, condition variables, or kernel notifications.
- Avoid sleep timeouts and repeated timed waits, parks, or yields used to poll state. Subscribe before checking, recheck after notifications, and wake blocked workers on shutdown or disconnect.
- Use timers for sampling schedules, pacing, simulated work, and real deadlines. Name fixed durations; derive deadlines from the operation they bound.

## I-06 — Limits and observations

- Document supported limits, scope, and exhaustion behavior. Limits, platform constraints, and caller budgets fail explicitly or expose documented partial results.
- Previews and pagination preserve access to the supported underlying result. Label partial observations; failed or truncated traversal cannot establish completeness, empty success, or deletion.
- Test relevant limits at acceptance and exhaustion. Support completeness claims with evidence that all supported pages or records are processed.

## I-07 — Behavior tests

- Assert documented behavior and observable outcomes, using real integration inputs where contracts exist. Avoid implementation assumptions and reproducing the logic under test.
- Generally use explicit `// Arrange`, `// Act`, and `// Assert` sections; repeat Act/Assert for state transitions.

## I-09 — API documentation

- Keep API contracts in docstrings/Rust doc comments beside code: meaning, behavior, constraints, parameters/results, errors, ownership, concurrency, and cancellation as applicable, with manpage precision. Keep docstrings at most three lines; use named sections or bullets when more detail is needed. Link READMEs to API docs.
- In functions with lengthy business logic, prefer concise, one-line inline comments explaining the key steps.

## I-10 — Step execution

- Invoke steps through the executor with owned, JSON-serializable inputs and outputs. Inject live services through registered factories, preserve input/output provenance and retain cleanup ownership through failure and cancellation. See the [step execution contract](../libs/workload-docker/src/workflows/README.md).
