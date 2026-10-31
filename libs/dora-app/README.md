# Dora app runtime

Shared Dora attachment and readiness for independently launched applications.
The [`environment`](src/environment.rs) module owns the approved environment
bridge and publishes readiness after application resources are initialized.

```rust,ignore
let (_node, mut events) = dora_app::environment::connect()?;
// Initialize application sinks and listeners before publishing readiness.
dora_app::environment::ready()?;
```

Enable the `node` feature for attachment. See the module's API documentation for
supported inputs, ownership and errors.
