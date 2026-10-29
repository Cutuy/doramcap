# Session

One bounded execution attempt, identified across workloads, recordings, and results.
An optional experiment ID groups original captures and comparative runs.
This crate will define identity, membership, evidence and outcome contracts. The
[workload lifecycle coordinator](../workload-lifecycle/) drives transitions and
manages participants, reporting their outcomes to the session.

![Session architecture](docs/session-architecture.svg)

Concept example; manifest serialization and Rust APIs remain future work:

```yaml
session_id: <unique-attempt-id>
experiment_id: <optional-comparison-group>
mode: target_replay
source_session_ids: [<original-capture-id>]
```

[Concept and integration boundaries](docs/102526-session-concept.md).
The crate currently contains documentation and has no dependencies.
