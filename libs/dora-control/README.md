# Dora coordinator control

`DoraCoordinatorClient` owns one experiment's reusable async WebSocket connection
and retained graph identity. Lifecycle steps share it through typed factory
injection; the backend owns native teardown.

```rust,ignore
let mut client = DoraCoordinatorClient::new(coordinator_endpoint, explicit_token);
client.wait_for_daemon(startup_deadline).await?;
```

See the [architecture](docs/110926-coordinator-client.md),
[implementation and readiness TODO](docs/110926-coordinator-client-impl.md), and
Rust API documentation beside `DoraCoordinatorClient`.
