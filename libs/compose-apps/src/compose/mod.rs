//! Docker Compose project configuration, runtime inspection and lifecycle steps.
//!
//! A `ComposeProject` is launch configuration; a `ResolvedComposeProject` is a
//! frozen execution manifest with run-specific identity. Neither owns running
//! containers through Rust destruction. Callers explicitly stop attempted launches.
pub mod adapter;
pub mod command;
pub mod error;
pub mod filesystem_preparation;
pub use filesystem_preparation::{ComposePrepareInput, PrepareOverlays, PreparedOverlays};
mod model;
pub(crate) mod mounts;
mod state;
mod steps;
/// Diagnostic operation for a service killed by SIGKILL or the OOM killer during stop.
pub const FORCED_STOP_OPERATION: &str = "forced stop";
pub use adapter::{ComposeExecution, DockerAdapter};
pub use error::{Error, Result};
pub(crate) use model::ResolvedBind;
pub use model::{
    Cleanup, ComposeProject, ComposeProjectState, ComposeService, ConfigurationSnapshot, Container,
    Diagnostic, ResolveInput, ResolvedComposeProject, ServiceExit, Shutdown, StartInput, StopInput,
    StoppingService,
};
pub use steps::{
    Diagnostics, InspectCompose, QuiesceCompose, RemoveCompose, ResolveCompose, StartCompose,
    StopCompose,
};
