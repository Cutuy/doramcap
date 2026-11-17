# Holoscan workload

Reads Holoscan CLI 5.0 catalogs and reviews source, native input and Build-artifact
facts. [Native Build](src/build.rs) streams disk logs, observes the retained builder
image and publishes [latest attempts](src/builds.rs) under repository admission.
Native launch details remain partial; application Start is later work.

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
[owned Docker options](src/cli_options.rs),
[preparation](src/preparation.rs), [container environment](src/environment.rs),
[CLI process ownership](src/process.rs) and the [runner plan](https://github.com/Cutuy/doramcap/pull/264).
Console's typed native client exposes `catalog(DiscoverRequest)` and
`build(BuildRequest)`, `buildLookup(BuildStoreRequest)` and
`inspect(RunRequest { run, holoscan: PrepareRequest })`. Progress and completion retain
backend-derived disk-log paths; CLI5 dollar expansion and NUL in owned Docker values
are rejected. Inspection loads current eligible Build evidence from `build_store_root`. Holoscan inspection returns
`Preparation` with verified snapshot inputs, retained CLI diagnostics, unknowns and
required-policy rejections. It requires explicit host-shm PASSTHROUGH and supports
application-only Review.

Run the real CLI checks from this crate:

```sh
node scripts/discovery-e2e.mts /absolute/holoscan /absolute/holoscan-applications
node scripts/project-inputs-e2e.mts /absolute/holoscan /absolute/holoscan-applications
```

After building the release native addon and generating Console messages, run the
read-only real CLI/Engine check with an existing source fixture:

```sh
../../console/server/node_modules/.bin/tsx scripts/prepare-e2e.mts /absolute/holoscan /absolute/project application /absolute/docker.sock
```

The capability-required native Build probe copies the small first_pipeline fixture,
uses its native Module base-image setting and retains output/evidence in a new folder:

```sh
../../console/server/node_modules/.bin/tsx scripts/build-e2e.mts /absolute/holoscan /absolute/first-pipeline-fixture /absolute/docker.sock cached-base-image /absolute/new-output
```
