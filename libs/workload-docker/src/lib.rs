//! Docker-specific lifecycle implementation and procedural workflows.
//!
//! [`compose`] provides project configuration, resolution, Engine integration and
//! lifecycle steps. [`workflows::recorded_compose`] assembles those steps with a
//! fresh CPU monitor and recording policy. Callers supply launch configuration
//! through Rust or serialized requests; no particular UI or input producer is required.
//! Generic step execution is supplied by the independent `orchestrator` crate.
pub mod compose;
pub mod evidence;
mod proto;
pub mod workflows;

/// Binary filesystem record schema; files contain length-delimited messages.
pub mod filesystem_proto {
    include!(concat!(env!("OUT_DIR"), "/doramcap.filesystem.v1.rs"));
}
