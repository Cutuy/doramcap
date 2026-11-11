# Console and open source alternatives

Created and source checked: 2026-11-11. Draft for the hosted doramcap website.

Console runs existing Docker Compose workloads as recorded experiments. It brings
workload lifecycle, recordings, setup and machine facts, and A/B comparison into
one workflow. The management interface supports those experiments.

## Comparison

| Solution | Main focus | Existing capabilities | Work needed for Console's experiment workflow |
| --- | --- | --- | --- |
| **Console** | Recorded workload experiments | Review resolved services, start supporting apps before the workload, capture recordings and machine facts, inspect past runs, and compare two runs | Broader platform qualification and richer performance analysis remain ongoing |
| **Dockge 1.5.0** — MIT | Compose project management | Compose editing, lifecycle controls, logs, terminals, and multiple agents | Experiment admission, readiness policy, capture, recovery, and run comparison remain custom; its deploy method runs detached Compose startup. [Source](https://github.com/louislam/dockge/blob/1.5.0/backend/stack.ts#L209-L215) |
| **Komodo 2.3.4** — GPL-3.0 | Deployment and automation across servers | Compose stacks, service operations, pre/post deployment hooks, procedures, monitoring, and execution logs | Experiment state and evidence handling remain custom. Procedures stop after a failed stage, so a later cleanup stage is skipped; post-deploy hooks run only after successful deployment. [Procedures](https://github.com/moghtech/komodo/blob/v2.3.4/bin/core/src/helpers/procedure.rs#L42-L105), [hooks](https://github.com/moghtech/komodo/blob/v2.3.4/bin/periphery/src/api/compose.rs#L759-L782) |
| **Portainer CE 2.45.3** — zlib | Container and stack administration | Web management of stacks, containers, images, networks, and volumes | Run capture, history, and comparison remain custom. Its ordinary Compose stack-stop path removes containers, so harvesting container evidence before removal requires a separate sequence. [Stop path](https://github.com/portainer/portainer/blob/2.45.3/api/http/handler/stacks/stack_stop.go#L135-L144), [removal](https://github.com/portainer/portainer/blob/2.45.3/api/stacks/deployments/deployer.go#L81-L87) |
| **Testcontainers Node 12.2.1** — MIT | Programmable disposable environments | Compose service selection, profiles, readiness strategies, container handles, and separate stop/down operations | Setup capture, filesystem policy, recovery, and A/B remain custom. Failed readiness triggers removal of containers and volumes, including when process-exit cleanup registration is disabled; retaining failed-run evidence requires changing that policy. [Compose API](https://node.testcontainers.org/features/compose/), [published source](https://github.com/testcontainers/testcontainers-node/blob/a3465510f5cc460a0fac1e696ad83781452abdfd/packages/testcontainers/src/docker-compose-environment/docker-compose-environment.ts) |

This compares the open source editions and the inspected lifecycle paths.
Their extension points can support additional behavior. Console's strength is
the combined experiment workflow; this table does not establish exclusive
capabilities or a measured implementation-cost advantage.

## Console's strengths

- **Adopt an existing Compose workload.** Review resolved services and select
  supporting apps without changing application code.
- **Give the experiment a defined lifetime.** Start infrastructure and recording
  apps before the workload, stop groups in reverse order, and gate removal on
  stopped writers and durable filesystem evidence. Failed or cancelled startup
  still enters teardown.
- **Keep the context behind a result.** Associate recordings with source,
  images, configuration digests, and observed machine configuration and
  condition. Execution, recording validity, and cleanup have separate outcomes.
- **Compare two past runs in one place.** Show differences in setup, machine
  facts, run status, duration, and recording results. Run again returns through
  Review so the current setup can be inspected.
- **Handle filesystem state explicitly.** Apply the configured policy to supported
  writable directory binds and retain filesystem-change evidence. Reset stops
  owned writers before discarding interrupted overlay state.

Implementation: [Compose lifecycle](../../libs/workload-docker/README.md),
[shared run sequence](../../libs/workload-lifecycle/src/run.rs),
[history](110226-experiment-history.md),
[machine capture and comparison](110426-machine-capture-comparison.md),
[filesystem policies](../../libs/workload-docker/docs/102626-docker-experiment-isolation.md).

## Proposed website copy

**Run your Compose workload. Keep the evidence. Compare the change.**

Console turns an existing Docker Compose project into a recorded experiment.
Review the setup, select recording and monitoring apps, and follow the run.
Keep its recordings and machine facts, then compare two past runs to see what
changed in the setup and results.

- Start supporting apps before the workload and track shutdown and cleanup.
- Keep each run's recordings alongside its setup and machine facts.
- Review differences between two runs and inspect the evidence behind them.

## Claim boundaries

Machine capture observes configuration; it does not restore the machine.
The current A/B view compares captured facts and basic results; statistical
performance-regression analysis is additional work. Run again reuses selections
and reviews current inputs rather than restoring the entire previous execution.

Filesystem behavior depends on the selected policy. Default write-back carries
host-file changes into the next run. Supported bind overlays and retained output
do not provide a coordinated snapshot of external services, arbitrary process
memory, or GPU state. Platform and failure-path qualification remains scoped to
the evidence recorded in the linked documents.

## Reuse assessment

For the current experiment behavior, the source review favors retaining
Compose/Bollard and Console's controller over integrating a full management
application. The experiment policy and evidence handling survive either route;
integration adds another resource model and upgrade boundary. Testcontainers is
a plausible source of narrower command/readiness reuse if its failure policy is
adapted. This is an engineering assessment, with no measured person-week or
total-cost estimate.

Before adding this material to the hosted website, review the public wording
and recheck the compared releases. The proposed copy above is the starting point;
the implementation assessment and detailed claim boundaries support that review.
