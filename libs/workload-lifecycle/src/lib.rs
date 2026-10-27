//! Mechanism-neutral workload lifecycle design.
//!
//! Lifecycle concerns include launch ownership, readiness, orderly shutdown,
//! recording completion and admission of the next run. Shared runtime contracts
//! will be extracted when multiple backends require the same semantics.
//!
//! Concrete Docker project configuration, steps and workflows live in the
//! `workload-docker` crate. Generic step execution lives in `orchestrator`.
//! This crate currently contains design documentation and no runtime API.
