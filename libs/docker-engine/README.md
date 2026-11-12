# Docker Engine operations

Share Docker connections, cleanup of owned resources and Docker storage observations across workload backends.

![Docker operations](architecture.svg)

```rust,ignore
let engine = docker_engine::Engine::connect("/var/run/docker.sock").await?;
let owned = engine.resources(controller_label, reservation_name);
let stopped = owned.quiesce(stop_seconds).await?;
```

Backends resolve mounts, verify container ownership and choose the container removed last.
Docker verifies socket-peer mount visibility, discovers volumes and streams stopped-container layer diffs into shared filesystem records.

See the [API](src/lib.rs), [Docker observations](src/filesystem.rs), [namespace check](src/namespace.rs) and [shared Linux capture](../host-filesystem/README.md).
