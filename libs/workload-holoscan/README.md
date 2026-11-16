# Holoscan workload

Reads application catalogs through the Holoscan CLI 5.0 JSON interface and
checks that native Docker routing selects Console's local daemon.

```rust,ignore
use orchestrator::Step;
use workload_holoscan::{DiscoverApplications, DiscoverRequest};
let catalog = DiscoverApplications::default().run(DiscoverRequest {
    project_root: "/projects/holoscan-applications".into(),
    cli: "/opt/holoscan/bin/holoscan".into(),
}).await?;
```

Console's typed native client exposes `catalog(DiscoverRequest) -> Catalog`.
See [`DiscoverApplications`](src/discovery.rs) and the
[runner plan](https://github.com/Cutuy/doramcap/pull/264). Build, preparation and
execution follow in later milestones. The real discovery check is
`node scripts/discovery-e2e.mts /absolute/holoscan /absolute/holoscan-applications`
from this crate.

## Native Docker routing

[`routing::native_docker_socket`](src/routing.rs) reads the native Linux routing
inputs with the exact CLI child working directory. A nonempty `DOCKER_HOST`
selects the virtual default context; otherwise `DOCKER_CONTEXT`, then
`config.json.currentContext`, then `default` selects the context.

| Input | Use |
| --- | --- |
| `DOCKER_CONFIG`, `HOME` | Configuration directory; relative paths use the CLI child's working directory. An absent/empty `HOME` uses the account home, as Docker does. |
| `DOCKER_HOST` | Native host override. Only absolute Unix sockets are supported. |
| `DOCKER_CONTEXT` | Native context name when no host override is set. |
| `DOCKER_TLS`, `DOCKER_TLS_VERIFY` | Nonempty values make an unspecified default host TCP, which is rejected. Explicit Unix endpoints remain Unix. |
| Selected JSON | Only `currentContext` or `Endpoints.docker.Host`; context metadata lives under Docker's SHA-256 directory. Authentication fields are ignored. |

Missing default configuration uses native defaults. Unreadable or malformed
selected routing fields fail with their file and JSON position. No Docker
subprocess, credential helper, certificate read or environment change is used.

[`routing::verify_native_daemon`](src/routing.rs) keeps the existing Unix-peer
mount-namespace check and reads both daemon IDs through the Engine API. Missing,
empty or different IDs block launch. Callers recheck before side effects when
inputs may have changed. These helpers prepare the later Build/Start workflow;
the catalog operation keeps its existing behavior.

Compatibility follows [Docker 29.9.1 routing source](https://github.com/docker/cli/blob/dfc520dcc28994c4c699b9b219fd2f5a6bcec0ff/cli/command/cli.go#L456)
and the same host-first order in 28.0.3. Installed 29.9.1 was checked with both
variables set. The [Docker CLI reference](https://docs.docker.com/reference/cli/docker/)
describes the opposite environment precedence; this adapter follows the verified
implementation. The supported CLI/executable must preserve native Docker routing.
Wrapper-injected flags/environment, remote transports and Docker Desktop/proxies
without the required namespace guarantee are outside this contract.
