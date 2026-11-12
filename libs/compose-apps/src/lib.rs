//! Compose project steps, Dora runtime control and shared system-app sequences.
//! Workflows own run order and finalization; these pieces preserve native Compose inputs.
pub mod cgroup;
pub mod compose;
pub mod dora;
mod evidence;
pub mod experiment;
pub mod preparation;
pub mod shared_dora;
pub mod steps;
pub mod system_apps;
