# Configuration and API reference

## Configuration

`Deployment` contains unique app IDs and typed `Policy` messages. Load a complete
standard protobuf TextFormat file with `config::load`; parsing preserves field
presence and rejects unknown fields. Every app requires explicit CPU placement.
An omitted RT priority has protobuf's numeric default, zero (`SCHED_OTHER`).

```rust
let deployment = resource_policy::config::load(path)?;
let goal = resource_policy::Goal::from_proto(deployment.app[0].policy.as_ref().unwrap())?;
let applied = resource_policy::linux::apply_current_thread(&goal)?;
```

## Caller APIs

| API | Contract |
|---|---|
| `inspect_current_thread` | Returns `ThreadResources`: affinity, scheduler, and priority. |
| `plan_current_thread` | Produces observed and desired thread state without mutation. |
| `apply_current_thread` | Changes differing fields and verifies convergence. Errors report partial progress and observed state. |
| `Instance::prepare` | Creates a new cgroup leaf with the goal CPU set under a caller-owned parent with cpuset already enabled. |
| `bootstrap_current_process` | Reconciles the owned cpuset, attaches the process, applies calling-thread policy, and verifies it. Requires single-threaded startup. |
| `Instance::remove` | Removes an empty owned leaf; failure preserves the handle for retry. |

## Bootstrap contract

- Apply policy before app initialization or dynamic loading that can run constructors.
- Coordinate thread creation and cgroup ownership exclusively. Pre-main constructors require earlier external enforcement.
- Cgroups contain processes; affinity and scheduling apply per thread. Subsequent workers normally inherit thread policy.
- Use separate processes for independently isolated app libraries.
- Handle partial failures explicitly. Kernel updates are non-atomic; recovery may require privileges.
- Manage cgroup cleanup explicitly. Dropping an instance preserves it; populated groups prevent removal.
- Reapply process bootstrap to reconcile its owned cpuset and calling-thread affinity to the explicit goal.
- Reinspect after hotplug or external changes. CPU placement permits sharing; RT requires compatible kernel/controller configuration and permissions.

Goal values come entirely from the supplied configuration. Observed state determines
which changes to apply; it never supplies desired CPU placement.

Parsing and enforcement allocate. Perform setup outside timing-sensitive loops;
reconciliation is caller-driven.

## Deployment adapters

| Target | Current support |
|---|---|
| systemd | `systemd_service_directives` emits CPU and scheduler settings for a fresh `[Service]` section. Callers supply executable, lifecycle, permissions, and deployment. |
| Kubernetes | Standard-target validation rejects exact CPU IDs and RT goals. Node/runtime adapters remain future work. |

Under an orchestrator, inspect externally applied policy and change only delegated
settings. GPU, bandwidth, budgets, exclusivity, and continuous reconciliation remain
future scope. Test commands and platform evidence live in [validation](validation.md).
