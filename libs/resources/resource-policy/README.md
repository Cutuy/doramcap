# Resource policy

CPU affinity, RT scheduling, and cgroup bootstrap for Linux app entrypoints.
Standard protobuf TextFormat supplies goal state; APIs apply and verify it.
systemd resource directives can be generated from the same policy.

```protobuf
app {
  id: "perception"
  policy { placement { cpus: "4,5-7,9-11" } rt_priority: 30 }
}
```

`placement.cpus` is required and uses Linux cpulist notation. Priority `0` selects `SCHED_OTHER`;
`1–99` selects `SCHED_RR`. Bootstrap runs before app initialization and worker creation.

```sh
cargo run --release --locked -p resource-policy --example resolve -- \
  libs/resources/resource-policy/configs/lab.pbtxt perception recorder
cargo test --release --locked -p resource-policy
```

- [Schema](proto/policy.proto) · [Profiles](configs/) · [API and configuration reference](docs/reference.md)
- [Implementation plan](docs/101926-resource-policy.md) · [x86_64 / ARM64 validation](docs/validation.md)
