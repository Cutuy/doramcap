//! Bounded output provenance and event-driven, anchored pair measurements.
//! See the component README for ordering, clock and retention contracts.
mod engine;
mod store;
pub use engine::{Engine, Lineage, Stats};
pub use store::{Limits, Record};

pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.provenance.v1.rs"));
}
pub const VERSION: u32 = 1;
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/provenance.bin"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;
fn invalid(message: impl Into<String>) -> Error {
    Error(message.into())
}

impl pb::GenerationRef {
    pub fn step_key(&self) -> pb::Step {
        pb::Step {
            producer: self.producer.clone(),
            name: self.step.clone(),
        }
    }
}
