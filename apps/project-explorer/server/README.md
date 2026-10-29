# Gallery hosting server

![Hosting architecture](docs/102727-on-demand-apps.svg)

Runs registered apps on demand and shares each instance among viewers. Caddy
routes app traffic; Docker Compose owns each generation's containers and network.
The server owns singleton admission, viewer leases and result publication.
Apps own `deploy/` directories containing their Compose files, images and
`registration.pbtxt`; the [protobuf schema](proto/registration.proto) defines
the registration fields.
The shared `workload-docker` runner executes startup, readiness, health checks,
shutdown and filesystem reporting through Orchestrator. Gallery sessions select
existing mounts and supply their own Compose services; recorded experiments can
select the shared OverlayFS policy and CPU collector. Recovery uses the same
shared teardown and gallery result-publication path.

```sh
cargo build --locked -p project-explorer-server --bins
```

[Setup, operation and recovery](docs/hosting.md) · [Implementation plan](docs/102727-on-demand-apps.md)

[Remaining apps, ordered by difficulty](docs/102727-remaining-apps.md)

[Designing new Docker-hostable apps](docs/docker-hostable-apps.md)

[Server modules and request path](docs/server-structure.md)
