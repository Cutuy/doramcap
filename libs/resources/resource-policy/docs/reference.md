# Configuration and API reference

## Composition

[`pbtxt-config`](../../../pbtxt-config/README.md) owns includes, App/Fragment/Deployment,
provenance, and hashing. Resource policy supplies validation and overlay semantics.

| Setting | Behavior |
|---|---|
| `include` | Relative paths; ordered depth-first; local fields override includes. Repeated includes reapply cached input. |
| App ID | Must exist in the caller's registry; duplicate declarations within a fragment fail. |
| `placement.cpus` | Decimal IDs and inclusive ranges; overlaps normalize; each assignment replaces the selection. |
| Complete goal | Every app requires nonempty `placement.cpus` after composition. Missing placement fails validation. |
| `rt_priority` | Omitted fields inherit, then default to 0. Explicit 0 overrides an inherited RT priority. |
| Invalid input | Unknown fields, invalid values, missing files, and include cycles fail resolution. |
| Provenance | Source bytes and assignment history are retained; semantic errors identify file/app. |
| Fingerprint | SHA-256 of normalized, versioned Protobuf: sorted apps, canonical cpulists, explicit defaults. |

Bounds: CPU IDs ≤1,048,575; cpulist ≤64 KiB and 1,048,576 expanded entries;
32 include levels; 256 visits; 1 MiB/file; 8 MiB total. Stride/group syntax is
unsupported. Semantic diagnostics currently omit line numbers.

## Caller APIs

```rust
let resolved = resource_policy::config::resolve(path, &["perception"])?;
let goal = resource_policy::Goal::from_proto(&resolved.config("perception")?)?;
let applied = resource_policy::linux::apply_current_thread(&goal)?;
```

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

Goal values come entirely from resolved configuration. Observed state determines
which changes to apply; it never supplies desired CPU placement. Partial fragments
may omit placement so another fragment supplies it.

Resolution and enforcement allocate. Perform setup outside timing-sensitive loops;
reconciliation is caller-driven.

## Deployment adapters

| Target | Current support |
|---|---|
| systemd | `systemd_service_directives` emits CPU and scheduler settings for a fresh `[Service]` section. Callers supply executable, lifecycle, permissions, and deployment. |
| Kubernetes | Standard-target validation rejects exact CPU IDs and RT goals. Node/runtime adapters remain future work. |

Under an orchestrator, inspect externally applied policy and change only delegated
settings. GPU, bandwidth, budgets, exclusivity, and continuous reconciliation remain
future scope. Test commands and platform evidence live in [validation](validation.md).
