# pbtxt-config

Protobuf TextFormat composition for app configuration. Owns the shared
App, Fragment, and Deployment schema; payloads use protobuf `Any`.

`resolve::<P>(path, registry)` loads ordered relative includes, validates every
fragment, and returns normalized app configs, provenance, source bytes, and a digest.
Implement `Payload` for a protobuf type to define validation, overrides, and normalization.

- Local assignments follow includes; repeated includes reapply cached source bytes.
- Unknown app IDs, duplicates within a fragment, type mismatches, and cycles fail.
- Limits: 32 include levels, 256 visits, 1 MiB/file, 8 MiB total.
- Digests hash versioned deployment bytes with sorted apps and canonical type URLs.
  Adapters must provide deterministic normalized payload encoding, including map ordering.
- Each resolution uses one payload type. Mixed-type registries remain future work.

[Schema](proto/config.proto) · [Resource-policy adapter](../resources/resource-policy/src/config.rs)

```sh
cargo test --locked -p pbtxt-config
```
