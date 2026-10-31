//! Docker Compose project configuration, runtime inspection and lifecycle steps.
//!
//! A `ComposeProject` is launch configuration; a `ResolvedComposeProject` is a
//! frozen execution manifest with run-specific identity. Neither owns running
//! containers through Rust destruction. Callers explicitly stop attempted launches.
pub(crate) mod adapter;
pub(crate) mod command;
pub(crate) mod error;
pub mod filesystem;
mod model;
pub mod overlay;
pub(crate) mod planner;
mod state;
mod steps;
pub use adapter::{ComposeExecution, DockerAdapter};
pub use error::{Error, Result};
pub use model::{
    Cleanup, ComposeProject, ComposeProjectState, ComposeService, Container, Diagnostic,
    ResolveInput, ResolvedComposeProject, Shutdown, StartInput, StopInput,
};
pub use steps::{
    Diagnostics, InspectCompose, QuiesceCompose, RemoveCompose, ResolveCompose, StartCompose,
    StopCompose,
};
