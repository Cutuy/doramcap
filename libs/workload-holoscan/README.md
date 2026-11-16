# Holoscan workload

Reads application catalogs through the native Holoscan CLI 5.0 JSON interface
and provides local Docker routing checks for later Build/Start workflows.

```rust,ignore
use orchestrator::Step;
use workload_holoscan::{DiscoverApplications, DiscoverRequest};
let catalog = DiscoverApplications::default().run(DiscoverRequest {
    project_root: "/projects/holoscan-applications".into(),
    cli: "/opt/holoscan/bin/holoscan".into(),
}).await?;
```

See the [discovery API](src/discovery.rs), [native routing contract](src/routing.rs)
and [runner plan](https://github.com/Cutuy/doramcap/pull/264). Console's typed
native client exposes `catalog(DiscoverRequest) -> Catalog`.

Run the real discovery check from this crate with
`node scripts/discovery-e2e.mts /absolute/holoscan /absolute/holoscan-applications`.
