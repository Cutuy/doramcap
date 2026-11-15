# Dora app runtime

Shared Dora attachment for independently launched applications.
The [`environment`](src/environment.rs) module owns the approved environment bridge.

```rust,ignore
let (_node, mut events) = dora_app::environment::connect()?;
```

Enable the `node` feature for attachment. See the module's API documentation for
supported inputs, ownership and errors.
