# Fault Dora adapter

Publish bounded fault reports and repair snapshots through an application’s existing Dora event loop.

Use `Health` to register checks, publish updates with `tick`, and send final state with `finish_to`. See the [API](src/lib.rs) and [fault libraries](../README.md).
