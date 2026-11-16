# Holoscan workload

Reads applications, language variants and mode descriptions through an existing
Holoscan CLI 5.0 JSON interface. The CLI owns native project search settings.

```rust,ignore
use orchestrator::Step;
use workload_holoscan::{DiscoverApplications, DiscoverRequest};
let catalog = DiscoverApplications.run(DiscoverRequest {
    project_root: "/projects/holoscan-applications".into(),
    cli: "/opt/holoscan/bin/holoscan".into(),
}).await?;
```

See [`DiscoverApplications`](src/discovery.rs) for the API and
[the adoption plan](https://github.com/Cutuy/doramcap/pull/264) for the runner.
Console's typed native client exposes `catalog(DiscoverRequest) -> Catalog`.
Build, preparation and execution follow in later milestones.
