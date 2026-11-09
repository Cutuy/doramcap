# System configuration

Typed, read-only observation of host machine state and shared-memory metadata.
Shared-memory capture failures leave `host_shm` absent and populate `host_shm_error`
for the UI. Other missing facts keep their zero values.

```rust
let state = system_config::observe(Path::new("/var/lib/doramcap"));
```

- [Schema](proto/system_config.proto)
- [Plan](../../../console/docs/110426-machine-capture-comparison.md)
