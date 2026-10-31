//! Concrete procedural workflows built from Docker lifecycle steps.
pub mod compose_lifecycle;
pub mod dora;
pub mod recorded_compose;
pub mod steps;
pub mod system_apps;

/// Shared Dora experiment API; application deployments switch in the adoption PR.
pub mod shared;
