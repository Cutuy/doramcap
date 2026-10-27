//! Workload launch adoption and lifecycle contracts for doramcap.
//!
//! This concept scaffold documents importing foreign launch definitions,
//! coordinating execution, and verifying session boundaries. The `session`
//! component owns shared identity and evidence contracts; resource policy owns
//! resource goals. Runtime APIs, serialization, supervision, and backend adapters
//! remain proposed. This crate has no dependencies or runtime implementation.
//!
//! See `README.md` and `docs/102626-foreign-workload-adoption.md`.
