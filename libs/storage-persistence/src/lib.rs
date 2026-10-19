//! Bounded managed buffers and transactional file-offset reservations.
//! Each pool and its exclusive leases stay on one owner thread.
//! io_uring and MCAP integration follow the gated engineering plan.

pub mod file_offsets;
pub mod managed_buffers;
pub mod mcap;
pub mod writer;
