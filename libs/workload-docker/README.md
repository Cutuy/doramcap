# Docker workload lifecycle

Run one machine-exclusive experiment in a stable Compose project per repository.
Pre-run reset stops old writers, unmounts overlays and discards old uppers before
preparation. Unexpected cleanup errors block launch. Per-run outputs and retained
named volumes remain separate; reset never replays interrupted host writes.

Compose handles configuration and container launch; Bollard verifies ownership,
readiness and cleanup. Dora starts before system apps, followed by adopted apps.
See the [plan and test results](docs/102926-shared-dora-experiment.md).

![Current and planned containers and IPC](docs/102926-shared-dora-containers.svg)

```rust,no_run
# async fn example(spec: workload_docker::workflows::recorded_compose::ComposeRun) -> workload_docker::compose::Result<()> {
use std::rc::Rc;
use workload_docker::{compose::DockerAdapter, workflows::recorded_compose::ComposeRunner};
let docker = Rc::new(DockerAdapter::connect("/var/run/docker.sock").await?);
let mut manager = ComposeRunner::new(docker);
let report = manager.run(spec, async {
    let _ = tokio::signal::ctrl_c().await;
}, |event: &orchestrator::Event| {
    // Forward to an injected telemetry sink.
    let _ = event;
}).await?;
assert!(report.successful());
# Ok(()) }
```

[Runnable example](examples/compose_experiment.rs) ·
[Robotics test candidates](../workload-lifecycle/docs/102626-adoption-candidates.md) ·
[Filesystem metadata steps](docs/102626-filesystem-metadata.md) ·
[Experiment isolation design](docs/102626-docker-experiment-isolation.md) ·
[Direct write-back plan and coverage](docs/102726-direct-overlay-writeback.md)

Build the daemon image from `deploy/Dockerfile` with a trusted Dora 1.1.0 binary
named `dora` in the build context. Each run pins the resulting image ID.
User-defined networks are preserved; an `IPC silos` warning explains that Dora
1.1's localhost attachment requires host networking. IPC and `/dev/shm` remain shared.

API details: `cargo doc -p workload-docker --no-deps`.
With that image available, run native Docker tests: `python3 libs/workload-docker/scripts/compose-e2e.py --output /tmp/compose-test`.

Host overlays require Linux CAP_SYS_ADMIN, a local daemon sharing the manager mount
namespace, and rsync. Direct write-back rejects ambiguous hardlinks and overlay
redirect/metacopy features; ACL/xattr/capability fidelity is untested. Use
`filesystem.disposition: "discard"` to delete upper data without changing original
host directories. Named volumes retain their separate declared policy.
