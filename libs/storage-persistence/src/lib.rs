//! Growable managed buffers and transactional file-offset reservations.
//! Managed pools stay on one owner thread; producer-owned slabs support cross-thread handoff.
//! Contiguous MCAP reservations use incremental fixed-buffer registration.

pub mod file_offsets;
pub mod managed_buffers;
pub mod mcap;
pub mod writer;

pub mod slabs;

pub mod observability;

pub mod mapped;

mod timeline;
