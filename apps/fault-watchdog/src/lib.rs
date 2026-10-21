//! Configured fault aggregation host and isolated publication adapters.
//!
//! The host supplies monotonic time and Dora input. Sink workers own blocking I/O;
//! reporter callbacks and the aggregation core create no threads.
pub mod config;
pub mod engine;
pub mod sinks;
use std::{
    io::Read,
    time::{SystemTime, UNIX_EPOCH},
};
/// Obtain a nonzero process incarnation from the OS random source.
/// Returns an I/O error rather than reusing an identity when entropy is unavailable.
pub fn incarnation() -> std::io::Result<u128> {
    let mut bytes = [0; 16];
    loop {
        std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
        let id = u128::from_be_bytes(bytes);
        if id != 0 {
            return Ok(id);
        }
    }
}
/// Optional wall-clock publication metadata; domain deadlines use monotonic time.
pub fn unix_ns() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos()
        .try_into()
        .ok()
}
