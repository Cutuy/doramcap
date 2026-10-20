//! Bounded activity capture with embedded background recording.
mod capture;
mod pipeline;
pub mod sinks;
mod subscriber;
pub mod wire;
pub use capture::{
    ActivityContext, BufferSnapshot, ClockSample, Config, Handle, Losses, Registration,
};
pub use pipeline::{BatchView, Runtime, ShutdownReport, Sink, SinkReport};
pub use subscriber::ActivitySubscriber;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;
