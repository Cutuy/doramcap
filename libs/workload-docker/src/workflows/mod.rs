//! Concrete procedural workflows built from Docker lifecycle steps.
pub mod compose_lifecycle;
pub mod dora;
mod observation;
pub mod project_steps;
pub mod recorded_compose;
pub mod recovery;
pub mod run_finalization;
pub mod run_preparation;
pub mod steps;
pub mod system_apps;
