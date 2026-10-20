//! Uncompressed, index-free MCAP framing. Payload encoding belongs to the caller.
//!
//! ```no_run
//! use storage_persistence::{mcap::{McapWriter, McapConfig, Schema, Channel},
//!     observability::{Collector, Mode, McapMonitor}};
//! # fn record(file: std::fs::File, schema: Schema<'_>, channels: &[Channel<'_>]) -> std::io::Result<()> {
//! let events = Collector::new(1024, Mode::Inspect)?;
//! let (metrics, monitor) = McapMonitor::channel();
//! let writer = McapWriter::new(file, schema, channels, McapConfig {
//!     observer: events.observer(), metrics, ..Default::default()
//! })?;
//! // Record through writer; sample through monitor; export outside the writer.
//! let stages = monitor.snapshot().unwrap();
//! writer.finish()?;
//! let final_stages = monitor.snapshot().unwrap();
//! # Ok(())
//! # }
//! ```
mod core;
use crate::{
    observability::Observer,
    slabs::{Rejected, SlabPool, WritableSlab},
};
use core::McapCore;
pub use core::{LargeReservation, MessageReservation, PreparedBlock, PreparedRecordsBuilder};
use std::{fs::File, io, time::Instant};
pub struct Schema<'a> {
    pub name: &'a str,
    pub encoding: &'a str,
    pub data: &'a [u8],
}
pub struct Channel<'a> {
    pub id: u16,
    pub topic: &'a str,
    pub encoding: &'a str,
}
#[derive(Clone, Copy)]
pub struct Message {
    pub channel: u16,
    pub sequence: u32,
    pub log_time: u64,
    pub publish_time: u64,
}
/// Recording configuration and injected observation dependencies.
/// `alignment = 1` uses buffered I/O; 512/4096 require an O_DIRECT file.
pub struct McapConfig {
    pub buffers: u32,
    pub buffer_size: usize,
    pub alignment: usize,
    pub io: crate::writer::WriterConfig,
    pub observer: Observer,
    pub metrics: crate::observability::McapMetrics,
}
impl Default for McapConfig {
    fn default() -> Self {
        Self {
            buffers: 8,
            buffer_size: crate::managed_buffers::DEFAULT_BUFFER_SIZE,
            alignment: 1,
            io: Default::default(),
            observer: Default::default(),
            metrics: Default::default(),
        }
    }
}

/// MCAP recording and lifecycle. Observation is injected through `McapConfig`.
pub struct McapWriter {
    core: McapCore,
}
impl McapWriter {
    pub fn new(
        file: File,
        schema: Schema<'_>,
        channels: &[Channel<'_>],
        config: McapConfig,
    ) -> io::Result<Self> {
        Ok(Self {
            core: McapCore::new(file, schema, channels, config)?,
        })
    }
    fn update<T>(&mut self, operation: impl FnOnce(&mut McapCore) -> T) -> T {
        let result = operation(&mut self.core);
        self.core.publish_metrics();
        result
    }
    pub fn reserve(
        &mut self,
        message: Message,
        payload_len: usize,
    ) -> io::Result<MessageReservation<'_>> {
        self.core.reserve(message, payload_len)
    }
    pub fn reserve_large(
        &mut self,
        message: Message,
        payload_len: usize,
    ) -> io::Result<LargeReservation<'_>> {
        self.core.reserve_large(message, payload_len)
    }
    pub fn prepare_records(&self, slab: WritableSlab) -> PreparedRecordsBuilder {
        self.core.prepare_records(slab)
    }
    pub fn append_prepared_records(
        &mut self,
        block: PreparedBlock,
    ) -> Result<crate::file_offsets::FileRange, Rejected<PreparedBlock>> {
        self.update(|c| c.append_prepared_records(block))
    }
    pub fn append_external_payload(
        &mut self,
        message: Message,
        lease: crate::mapped::ReadLease,
    ) -> Result<crate::file_offsets::FileRange, Rejected<crate::mapped::ReadLease>> {
        self.update(|c| c.append_external_payload(message, lease))
    }
    pub fn append_external_records(
        &mut self,
        lease: crate::mapped::ReadLease,
    ) -> Result<crate::file_offsets::FileRange, Rejected<crate::mapped::ReadLease>> {
        self.update(|c| c.append_external_records(lease))
    }
    pub fn prepare_slabs(&mut self, pool: &SlabPool) -> io::Result<()> {
        self.update(|c| c.prepare_slabs(pool))
    }
    pub fn retire_slabs(&mut self, pool: &SlabPool) -> io::Result<()> {
        self.update(|c| c.retire_slabs(pool))
    }
    pub fn prepare_region(&mut self, region: &crate::mapped::MmapRegion) -> io::Result<()> {
        self.update(|c| c.prepare_region(region))
    }
    pub fn retire_region(&mut self, region: &crate::mapped::MmapRegion) -> io::Result<()> {
        self.update(|c| c.retire_region(region))
    }
    pub fn trim(&mut self) -> io::Result<usize> {
        self.update(McapCore::trim)
    }
    pub fn next_flush_deadline(&self) -> Option<Instant> {
        self.core.next_flush_deadline()
    }
    pub fn poll(&mut self, now: Instant) -> io::Result<()> {
        self.update(|c| c.poll(now))
    }
    pub fn flush_written(&mut self) -> io::Result<()> {
        self.update(McapCore::flush_written)
    }
    pub fn finish(mut self) -> io::Result<()> {
        self.update(McapCore::seal)
    }
    pub fn rotate(&mut self, file: File) -> io::Result<()> {
        self.update(|c| c.rotate(file))
    }
    pub fn file_bytes(&self) -> u64 {
        self.core.file_bytes()
    }
    pub fn max_inline_payload(&self) -> usize {
        self.core.max_inline_payload()
    }
    pub fn max_large_payload(&self) -> usize {
        self.core.max_large_payload()
    }
}
