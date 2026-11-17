# Holoscan workload

Reads native Holoscan CLI 5.0 catalogs and raw Module Docker inputs, and checks
local Docker routing for later Build/Start workflows.

```rust,ignore
use orchestrator::Step;
use workload_holoscan::{DiscoverApplications, DiscoverRequest};
let catalog = DiscoverApplications::default().run(DiscoverRequest {
    project_root: "/projects/holoscan-applications".into(),
    cli: "/opt/holoscan/bin/holoscan".into(),
}).await?;
```

See the [discovery API](src/discovery.rs), [raw project inputs](src/project.rs),
[native routing contract](src/routing.rs) and [runner plan](https://github.com/Cutuy/doramcap/pull/264). Console's typed
native client exposes `catalog(DiscoverRequest) -> Catalog`.

Run the real CLI checks from this crate:

```sh
node scripts/discovery-e2e.mts /absolute/holoscan /absolute/holoscan-applications
node scripts/project-inputs-e2e.mts /absolute/holoscan /absolute/holoscan-applications
```
