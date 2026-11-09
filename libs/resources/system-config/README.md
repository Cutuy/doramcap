# System configuration

Typed, read-only observation of host machine configuration and condition.
Observation returns `Result<MachineState, String>`; errors identify unavailable captures.

```rust
let state = system_config::observe(Path::new("/var/lib/doramcap"))?;
```

- [Schema](proto/system_config.proto)
- [Plan](../../../console/docs/110426-machine-capture-comparison.md)
