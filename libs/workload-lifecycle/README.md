# Workload lifecycle

Mechanism-neutral design for workload ownership, readiness, recording completion
and verified shutdown. Runtime contracts will be extracted from concrete backends
when shared semantics are established.

The planned lifecycle coordinator drives session transitions and manages
participants through launch, readiness, shutdown and cleanup. [Session](../session/)
will define the identities, membership, evidence and outcomes recorded for those transitions.

- [Component boundaries](docs/102626-component-boundaries.md)
- [Generic step execution](../orchestrator/)
- [Docker implementation and recorded Compose workflow](../workload-docker/)
- [Lifecycle architecture and roadmap](docs/102626-foreign-workload-adoption.md)
- [Robotics adoption research](docs/102626-adoption-candidates.md)
