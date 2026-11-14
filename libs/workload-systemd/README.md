# systemd workload lifecycle

Run experiment apps as owned systemd units over D-Bus, with shared startup,
recording, writable-mount overlays, file-change reports and verified cleanup.

![systemd mode](../workload-lifecycle/docs/110726-systemd-slice.svg)

```sh
cargo build --release --locked -p workload-systemd --example systemd_run
sudo target/release/examples/systemd_run run.pbtxt plan.pbtxt
```

Inputs: [`SystemdPlan`](proto/systemd_plan.proto). Requires root, the system
service manager, OverlayFS and rsync. [Architecture](../workload-lifecycle/docs/110726-launch-forms.md) ·
[Native tests](tests/native.rs) · API: `cargo doc -p workload-systemd --no-deps`.

[Workflow-step design](../orchestrator/docs/102626-function-steps.md#choosing-workflow-steps) ·
[Overlays](docs/111326-systemd-overlays.md) · [Host checks](tests/overlays.mts).
