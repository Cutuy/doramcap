//! Application catalogs read through an existing Holoscan CLI 5 JSON interface.
//! Native project configuration owns search and metadata interpretation.
mod discovery;

pub use discovery::DiscoverApplications;
include!(concat!(env!("OUT_DIR"), "/doramcap.holoscan.v1.rs"));
