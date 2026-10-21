# Fault schema

Owns the `doramcap.fault.v1` Protobuf files, generated `pb` types and `FILE_DESCRIPTOR_SET`. Reporter and aggregation import these enums directly. [fault-protocol](../protocol/) handles checked conversion between bounded records and wire messages.

- [Reporter IPC](proto/reporter.proto)
- [Aggregation storage/live publication](proto/aggregation.proto)
- [Shared evidence](proto/common.proto)

`tests/baselines/v1.bin` freezes the v1 descriptor. Compatibility tests reject field/type/cardinality/oneof changes, enum renumbering and unreserved deletions. Preserve the baseline when extending v1; reserve deleted names and numbers. Intentional breaking changes require a new package/version and retained old-contract coverage.

```sh
cargo test --locked -p fault-schema
```
