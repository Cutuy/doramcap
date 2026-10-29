//! Server-owned coordination for on-demand gallery applications.

pub mod api;
mod artifact_fs;
pub mod lease;
pub mod lifecycle;
pub mod manager;
pub mod registry;
pub mod runtime;
pub mod workflow;
