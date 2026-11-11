# Apps in Compose containers

Project lifecycle steps, Dora runtime control and preparation of Dora, system and
optional extra app groups. Workflows supply their own run order and finalization.

![Shared apps](../workload-docker/docs/111126-compose-apps-crate.svg)

```rust,no_run
# async fn example(input: compose_apps::preparation::PrepareInput) -> compose_apps::compose::Result<()> {
use compose_apps::{compose::DockerAdapter, preparation::resolve};
let adapter = DockerAdapter::connect("/var/run/docker.sock").await?;
let resolved = resolve(&adapter, input).await?;
assert!(!resolved.plan.manifest.is_empty());
# Ok(()) }
```

[Architecture](../workload-docker/docs/111126-compose-apps-crate.md) ·
[Compose workflow](../workload-docker) · API: `cargo doc -p compose-apps --no-deps`.
