# systemd workload lifecycle

Run experiment apps as owned systemd units over D-Bus, with startup, recording
readiness and verified cleanup managed by the shared lifecycle.

![systemd mode](../workload-lifecycle/docs/110726-systemd-slice.svg)

```sh
cargo build --release --locked -p workload-systemd --example systemd_run
sudo target/release/examples/systemd_run run.pbtxt plan.pbtxt
```

Inputs: [`SystemdPlan`](proto/systemd_plan.proto). Requires root and the system
service manager. [Architecture](../workload-lifecycle/docs/110726-launch-forms.md) ·
[Native tests](tests/native.rs) · API: `cargo doc -p workload-systemd --no-deps`.

[Workflow-step design](../orchestrator/docs/102626-function-steps.md#choosing-workflow-steps) ·
[Runtime step implementation](docs/110826-runtime-steps-impl.md).
