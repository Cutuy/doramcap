# Holoscan workload

Reads Holoscan CLI 5.0 catalogs and reviews source, native input and Build-artifact
facts. Backend-owned [Build attempts](src/builds.rs) retain latest state and disk-log
locations under admission. Native launch details remain partial; Build invocation
and Start are later work.

```rust,ignore
use orchestrator::Step;
use workload_holoscan::{DiscoverApplications, DiscoverRequest};
let catalog = DiscoverApplications::default().run(DiscoverRequest {
    project_root: "/projects/holoscan-applications".into(),
    cli: "/opt/holoscan/bin/holoscan".into(),
}).await?;
```

See [discovery](src/discovery.rs), [project inputs](src/project.rs),
[native routing](src/routing.rs), [Build records](src/build_record.rs),
[preparation](src/preparation.rs), [container environment](src/environment.rs),
[CLI process ownership](src/process.rs) and the [runner plan](https://github.com/Cutuy/doramcap/pull/264).
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
