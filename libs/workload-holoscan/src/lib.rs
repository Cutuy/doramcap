//! Native Holoscan CLI discovery and local Docker routing checks.
//! Native project configuration owns search and metadata interpretation.
pub mod build_record;
mod discovery;
pub mod environment;
pub mod preparation;
pub mod project;
pub mod routing;

pub use discovery::DiscoverApplications;
pub use preparation::PrepareLaunch;
include!(concat!(env!("OUT_DIR"), "/doramcap.holoscan.v1.rs"));
