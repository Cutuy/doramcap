//! Growable managed buffers and transactional file-offset reservations.
//! Each pool and its exclusive leases stay on one owner thread.
//! Contiguous MCAP reservations use incremental fixed-buffer registration.

pub mod file_offsets;
pub mod managed_buffers;
pub mod mcap;
pub mod writer;
