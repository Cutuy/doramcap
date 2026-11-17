# Holoscan workload

Reads Holoscan CLI 5.0 catalogs and reviews verified source, native input and
Build-artifact facts. Unavailable native launch details remain explicitly unknown;
Build and Start are later milestones.

```rust,ignore
use orchestrator::Step;
use workload_holoscan::{DiscoverApplications, DiscoverRequest};
let catalog = DiscoverApplications::default().run(DiscoverRequest {
    project_root: "/projects/holoscan-applications".into(),
    cli: "/opt/holoscan/bin/holoscan".into(),
}).await?;
```

See the [discovery API](src/discovery.rs), [raw project inputs](src/project.rs),
[native routing contract](src/routing.rs), [Build-record assessment](src/build_record.rs),
[partial preparation API](src/preparation.rs), [container environment](src/environment.rs)
and [runner plan](https://github.com/Cutuy/doramcap/pull/264).
Console's typed native client exposes `catalog(DiscoverRequest)` and
`inspect(RunRequest { run, holoscan: PrepareRequest })`. Holoscan inspection returns
`Preparation` with verified snapshot inputs, retained CLI diagnostics, unknowns and
required-policy rejections. It requires explicit host-shm PASSTHROUGH and supports
application-only Review.

Run the real CLI checks from this crate:

```sh
node scripts/discovery-e2e.mts /absolute/holoscan /absolute/holoscan-applications
node scripts/project-inputs-e2e.mts /absolute/holoscan /absolute/holoscan-applications
```

After building the release native addon and generating Console messages, run the
read-only real CLI/Engine check with an existing source fixture and Build image:

```sh
../../console/server/node_modules/.bin/tsx scripts/prepare-e2e.mts /absolute/holoscan /absolute/project application sha256:image /absolute/docker.sock
```
