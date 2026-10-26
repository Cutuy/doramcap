//! Session identity and experiment evidence contracts for doramcap.
//!
//! A session identifies one bounded execution attempt and the evidence required
//! to track, record, reproduce, analyze, and query it. Replays receive new session
//! identities and reference their source sessions. An optional experiment identity
//! groups related attempts.
//!
//! This crate currently establishes the component and its concept documentation.
//! Runtime APIs, manifest serialization, identity generation, and tool adapters
//! remain future work. See `README.md` and `docs/102526-session-concept.md`.
