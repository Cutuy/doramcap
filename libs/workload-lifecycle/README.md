# Workload lifecycle

Adopt foreign launch definitions and coordinate execution within a doramcap session.
Native monitors/writers and foreign workloads live for one session. Only the
orchestration/lifecycle manager and its self-monitoring persist across experiments.
External daemons require explicit compatibility exceptions.

Design-only crate; execution APIs and adapters remain future work.

![Workload lifecycle architecture](docs/102626-foreign-workload-adoption.svg)

Conceptual use: import a Compose project, attach experiment policy, review the
resolved plan, then execute and verify cleanup under one session identity.

[Workflow chart](docs/102626-experiment-workflow.svg) ·
[Component interactions](docs/102626-experiment-components.svg) ·
[Design and milestones](docs/102626-foreign-workload-adoption.md) ·
[Robotics feasibility review](docs/102626-adoption-candidates.md) ·
[Session contracts](../session/README.md)
