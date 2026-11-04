# System configuration

Typed, read-only observation of host machine state. A missing or unreadable fact keeps its zero value, so `observe` never fails.

```rust
let state = system_config::observe(Path::new("/var/lib/doramcap"));
```

- [Schema](proto/system_config.proto)
- [Plan](../../../console/docs/110426-machine-capture-comparison.md)
