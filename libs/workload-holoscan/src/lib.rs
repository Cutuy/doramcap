//! Native Holoscan CLI discovery and local Docker routing checks.
//! Native project configuration owns search and metadata interpretation.
pub mod application;
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

#[cfg(test)]
mod test_support;

pub use discovery::DiscoverApplications;
pub use preparation::PrepareLaunch;
include!(concat!(env!("OUT_DIR"), "/doramcap.holoscan.v1.rs"));

/// Trusted Engine start evidence is present and excludes Docker's zero-time sentinel.
/// Timestamp syntax stays with Engine; this is not an RFC3339 parser.
pub(crate) fn container_started(started_at: Option<&str>) -> bool {
    started_at.is_some_and(|value| !value.is_empty() && !value.starts_with("0001-"))
}
