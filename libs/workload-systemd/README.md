# systemd workload lifecycle

Run one experiment's apps as owned systemd units in a per-repository slice, controlled
over D-Bus. Groups start in order and stop in reverse; repository `.service` files are
adopted unchanged. A failed unit is recorded and the run continues until stop.

![systemd mode](../workload-lifecycle/docs/110726-systemd-slice.svg)

```sh
cargo build --release --locked -p workload-systemd --example systemd_run
sudo target/release/examples/systemd_run plan.pbtxt
```

Inputs: [`SystemdPlan`](proto/systemd_plan.proto). Needs root and the system service
manager. Native tests: [tests/native.rs](tests/native.rs).
[Plan](../workload-lifecycle/docs/110726-launch-forms.md) · API: `cargo doc -p workload-systemd --no-deps`.
