//! Native Holoscan CLI discovery and local Docker routing checks.
//! Native project configuration owns search and metadata interpretation.
mod discovery;
pub mod routing;

pub use discovery::DiscoverApplications;
include!(concat!(env!("OUT_DIR"), "/doramcap.holoscan.v1.rs"));
