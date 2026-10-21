//! Run an actual CPU/GPU overlapping-read handshake and rejected-write checks.
mod support;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = if std::env::args().any(|arg| arg == "--managed") {
        cuda_host_memory::AllocationMode::Managed
    } else {
        cuda_host_memory::AllocationMode::Registered
    };
    support::concurrent_read(mode)
}
