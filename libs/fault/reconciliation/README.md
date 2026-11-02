# Fault reconciliation

Single-owner state for a fault host: binds inputs to reporter incarnations, tracks
source liveness, reports local diagnostics and produces aggregation frames.

The host owns configuration, the event loop, clocks and publication. The library
performs no I/O and creates no threads. API details are in the Rust documentation.
