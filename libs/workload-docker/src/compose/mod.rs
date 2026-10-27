//! Docker Compose project configuration, runtime inspection and lifecycle steps.
//!
//! A `ComposeProject` is launch configuration; a `ResolvedComposeProject` is a
//! frozen execution manifest with run-specific identity. Neither owns running
//! containers through Rust destruction. Callers explicitly stop attempted launches.
pub(crate) mod adapter;
mod command;
pub(crate) mod error;
mod model;
mod steps;
pub use adapter::DockerAdapter;
pub use error::{Error, Result};
pub use model::{
    Cleanup, ComposeProject, ComposeProjectState, ComposeService, Container, Diagnostic,
    ResolveInput, ResolvedComposeProject, StartInput, StopInput,
};
pub use steps::{Diagnostics, InspectCompose, ResolveCompose, StartCompose, StopCompose};
