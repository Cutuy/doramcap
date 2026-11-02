//! Configured fault aggregation host and isolated publication adapters.
//!
//! The host supplies monotonic time and Dora input. Sink workers own blocking I/O;
//! reporter callbacks and the aggregation core create no threads.
pub mod config;
pub mod sinks;
