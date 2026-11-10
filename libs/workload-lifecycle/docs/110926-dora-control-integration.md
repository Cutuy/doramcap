![Dora control integration](110926-dora-control-integration.svg)

# Dora control integration

Created: 2026-11-09. Updated: 2026-11-10. [Implementation plan](110926-dora-control-integration-impl.md).

[PR #251](https://github.com/Cutuy/doramcap/pull/251) provides the qualified
`DoraCoordinatorClient`. Two adoption PRs will replace transient Dora control
processes in Compose and systemd. Coordinator and daemon services keep their
native launch definitions, ownership and shutdown verification.

## Ownership and interaction

Each experiment owns one client from before its first graph-start attempt through
verified cleanup. Compose retains it in `ExperimentOwner`, already shared with
`LocalExecutor` step factories and `ComposeRunning`. systemd retains it in the
Dora state owned by `SystemdRunning`. Steps borrow that retained state; the existing
typed factories provide injection.

The executor connects directly to the owned coordinator's explicit endpoint.
Successive calls reuse its WebSocket. Backend preparation supplies the frozen
dynamic descriptor, unique run name, Dora session identity and runtime-visible
working directory. The client owns live transport, start intent and graph UUID;
the backend owns native processes, persisted evidence and removal permission.

| Existing control invocation | Direct client operation |
| --- | --- |
| `status --format json` during startup | `wait_for_daemon(startup_deadline)` |
| `start --detach`, then `list --name` | `start_graph(Start, startup_deadline)`, then validate the owned graph |
| `list --name` during observation | `inspect_graph(observation_deadline)` |
| `node stop --dataflow --grace` | `stop_node(node, grace, stop_deadline)` |
| `down --force` | `shutdown(stop_deadline)` after application drain |

Compose removes its control `exec` calls. systemd removes its `dora-cli-*`
oneshots and their `cli-*.out` files. Existing backend observation policies and
application drain order continue to apply.

## Workflow, cleanup and recovery

![Client calls by workflow phase and recovery path](110926-dora-control-workflow-recovery.svg)

## Failure and cleanup

Connect, daemon readiness and graph registration share one absolute startup
deadline derived from the existing budget. Cancellation discards the in-flight
socket while the retained client keeps start intent and any UUID already received.
Cleanup receives a fresh stop budget after cancelled startup. Node stops and
runtime shutdown share that stop deadline; native teardown follows existing
backend stop rules even when control fails.

Cleanup can reconnect and reconcile an uncertain Start by exact run name. Missing
or ambiguous identity stays unknown. Preserve that diagnostic, continue native
teardown and verify actual exit before removal. Mutations are never replayed
automatically. A matching stopped reply permits Destroy even when nodes failed;
report node and destruction errors together. Force shutdown requires exclusive
coordinator/daemon ownership.
After controller exit, the existing persisted run ownership and reset path recover
native resources. Socket closure alone never establishes graph or process exit.

## Adoption boundaries

Both backends support externally launched dynamic nodes. Preserve descriptor
validation, source/target paths, session artifacts and shared-memory attachment.
Map CLI preparation to typed Start fields explicitly; arbitrary graph builds are
outside this migration. Endpoint reachability and authentication must work from
the executor's context before enabling direct control. Introduce no environment
inputs or new dependencies.

The approved, bounded 100 ms daemon-registration polling remains inside the
client. Its [event-driven replacement TODO](../../dora-control/docs/110926-coordinator-client-impl.md#todo-replace-startup-readiness-polling)
continues separately. The [migration roadmap](https://github.com/Cutuy/doramcap/pull/210)
records the approved lifetime and deadline decisions.
