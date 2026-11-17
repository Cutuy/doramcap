//! Native Holoscan CLI discovery and local Docker routing checks.
//! Native project configuration owns search and metadata interpretation.
pub mod build;
pub mod build_record;
pub mod builds;
pub mod cli_options;
mod discovery;
pub mod environment;
pub mod preparation;
pub mod process;
pub mod project;
pub(crate) mod reset;
pub mod routing;

pub use discovery::DiscoverApplications;
pub use preparation::PrepareLaunch;
include!(concat!(env!("OUT_DIR"), "/doramcap.holoscan.v1.rs"));
