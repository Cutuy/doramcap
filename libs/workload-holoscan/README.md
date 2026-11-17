# Holoscan workload

Discovers, builds, reviews and runs existing applications through Holoscan CLI 5.0
and Docker. [Build](src/build.rs) publishes [latest attempts](src/builds.rs).
[Application Run](src/application/mod.rs) rechecks Review, uses shared overlays and
saves independent execution, filesystem and cleanup facts. Native launch details
remain partial.

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
Console's typed native client exposes catalog, Build, Build lookup, inspect and
Start. Inspection loads authoritative Build evidence and retains verified snapshot
facts, actual Compose system inputs, partial diagnostics and explicit unknowns. Start requires that snapshot and
host-shm PASSTHROUGH; [shared system phases](src/system/runtime.rs) precede the native application and save independent shutdown facts in the typed report and
shared file-change checkpoint.

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

[Native Run qualification](tests/application-real.mts) uses a retained Build store
and a fresh output directory with the discard policy.
