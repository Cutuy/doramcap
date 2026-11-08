# Design guide

Use this checklist before implementation and during design review. Each checked item should have a concrete answer or supporting evidence.

- [ ] **Define behavior before choosing mechanisms.** Agree on the user workflow, scope, success conditions, failure behavior, and recovery policy. Resolve important choices before encoding them across components.

- [ ] **Choose the smallest design that meets current requirements.** Justify every abstraction, configuration option, dependency, and compatibility layer with a concrete use case. Consider deletion and explicit configuration first.

- [ ] **Give every fact and policy one authoritative owner.** Define shared concepts once. Keep backend-specific details in adapters. Avoid duplicate models, defaults, state stores, and cross-language wrappers that merely forward operations.

- [ ] **Preserve native contracts.** Use existing platform formats and semantics where practical. Identify exactly which settings the application preserves, overrides, or rejects. Avoid reconstructing another system’s configuration model.

- [ ] **Design ownership across the entire lifecycle.** Identify who owns each resource from the first attempted side effect through verified cleanup. Specify behavior for partial startup, cancellation at every await, process failure, and controller crashes.

- [ ] **Keep facts, interpretations, and outcomes distinct.** Represent unknown explicitly. Evaluate execution, recording validity, and cleanup independently. Preserve useful evidence even when another part of the operation fails.

- [ ] **Make persistence an explicit product decision.** Specify what survives navigation, refresh, restart, and upgrade. Identify the authoritative store, atomic updates, retention policy, and behavior when saving fails.

- [ ] **Make reviewed inputs match executed inputs.** Share resolution logic between inspection and execution. Distinguish stable configuration from generated runtime values. Apply transformations once and detect meaningful changes before side effects.

- [ ] **Make progress and shutdown explicit.** Identify the signal for work arrival, capacity, completion, failure, and shutdown. Check subscription races and disconnected producers. Use timers only for defined schedules or deadlines.

- [ ] **Evaluate optimizations across the complete data path.** Measure realistic low-rate, burst, and saturated workloads. Account for allocations, retained memory, copies, ordering, latency, padding, and crash loss. Justify custom implementations with measured benefits.

- [ ] **Treat deployment boundaries as part of the design.** Specify process, filesystem, network, shared-memory, privilege, and dependency boundaries. Check concurrent instances and absent optional services. Verify that cleanup affects only owned resources.

- [ ] **Validate observable contracts in representative environments.** Exercise packaged applications, real platform behavior, and independent consumers. Include failure, cancellation, restart, and cleanup. Synchronize tests on observable events; distinguish implemented behavior from qualified behavior.
