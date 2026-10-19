# Resource policy

CPU affinity, RT scheduling, and cgroup bootstrap for Linux app entrypoints.
[`pbtxt-config`](../pbtxt-config/) composes pbtxt goal state; APIs apply and verify it. systemd resource
directives can be generated from the same policy.

```protobuf
include: "base.pbtxt"
app {
  id: "perception"
  config {
    [type.googleapis.com/resource_policy.v1.Policy] {
      placement { cpus: "4,5-7,9-11" } rt_priority: 30
    }
  }
}
```

`placement.cpus` is required and uses Linux cpulist notation. Priority `0` selects `SCHED_OTHER`;
`1–99` selects `SCHED_RR`. Bootstrap runs before app initialization and worker creation.

```sh
cargo run -p resource-policy --example resolve -- \
  libs/resource-policy/configs/lab.pbtxt perception recorder
cargo test --locked -p resource-policy
```

- [Schema](proto/policy.proto) · [Profiles](configs/) · [API and configuration reference](docs/reference.md)
- [Implementation plan](docs/101926-resource-policy.md) · [x86_64 / ARM64 validation](docs/validation.md)
