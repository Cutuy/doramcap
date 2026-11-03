//! Uncompressed, index-free MCAP framing. Payload encoding belongs to the caller.
//!
//! # Direct-I/O padding
//! Direct I/O uses [private MCAP records](https://mcap.dev/spec#records) (`0x80`) for alignment.
//! Message readers skip padding; record iterators may expose it as unknown records.
//! Padding increases disk usage and write traffic and can exceed one alignment block.
//!
//! # Example
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
///
/// Submissions drive I/O on the calling thread; periodic polling is optional.
/// Successful commits/appends mean acceptance. Failures discovered after acceptance
/// are retained for the next submission or explicit flush/finish barrier.
/// Idle batches remain buffered until another writer call. Rings require Linux 6.1+.
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
        let (mut schema_channel_records, mut content) = (Vec::new(), Vec::new());
        content.extend_from_slice(&1_u16.to_le_bytes());
        core::string(&mut content, schema.name.as_bytes())?;
        core::string(&mut content, schema.encoding.as_bytes())?;
        core::string(&mut content, schema.data)?;
        core::record(&mut schema_channel_records, 3, &content);
        for channel in channels {
            content.clear();
            content.extend_from_slice(&channel.id.to_le_bytes());
            content.extend_from_slice(&1_u16.to_le_bytes());
            core::string(&mut content, channel.topic.as_bytes())?;
            core::string(&mut content, channel.encoding.as_bytes())?;
            content.extend_from_slice(&0_u32.to_le_bytes());
            core::record(&mut schema_channel_records, 4, &content);
        }
        Self::with_records(file, &schema_channel_records, config)
    }
    /// Starts a recording on owned `file` from serialized [MCAP Schema and Channel records](https://mcap.dev/spec#records).
    /// Caller supplies valid declarations with at least one channel and unique channel IDs.
    /// Errors: invalid framing, record types, channel IDs or configuration; allocation and I/O failures.
    pub fn with_records(
        file: File,
        schema_channel_records: &[u8],
        config: McapConfig,
    ) -> io::Result<Self> {
        Ok(Self {
            core: McapCore::new(file, schema_channel_records, config)?,
        })
    }
    fn update<T>(&mut self, operation: impl FnOnce(&mut McapCore) -> T) -> T {
        let result = operation(&mut self.core);
        self.core.publish_metrics();
        result
    }
    /// Reserve one contiguous writable payload for `message`.
    ///
    /// Borrows this writer until the reservation is committed or dropped. Dropping
    /// an uncommitted reservation abandons its bytes. Invalid channels/sizes,
    /// allocation/registration failures, backpressure and pending I/O errors return
    /// errors. Commit establishes acceptance; use [`Self::finish`] for durability.
    pub fn reserve(
        &mut self,
        message: Message,
        payload_len: usize,
    ) -> io::Result<MessageReservation<'_>> {
        self.core.reserve(message, payload_len)
    }
    /// Copy `payload` into one message in call order, blocking through backpressure.
    /// Errors: invalid channels, allocation and retained I/O failures, and `WouldBlock`
    /// when registrations stay full after a flush. Success means acceptance.
    pub fn write(&mut self, message: Message, payload: &[u8]) -> io::Result<()> {
        // Large payloads bypass the combining buffer, so drain it first to keep order.
        if payload.len() > self.max_inline_payload() {
            self.flush_written()?;
        }
        let mut flushed = false;
        loop {
            match self.reserve(message, payload.len()) {
                Ok(mut reservation) => {
                    reservation.payload_mut().copy_from_slice(payload);
                    // Large payloads flushed above and small ones join the batch, so pending has room.
                    return reservation.try_commit();
                }
                // Flushing frees everything except registrations, so a second refusal is permanent.
                Err(error) if flushed || error.kind() != io::ErrorKind::WouldBlock => {
                    return Err(error);
                }
                Err(_) => {}
            }
            self.flush_written()?;
            flushed = true;
        }
    }
    /// Reserve a contiguous payload through the compatibility `BufMut` adapter.
    /// Uses the same placement, ownership and errors as [`Self::reserve`]; small
    /// payloads may share a combining buffer. The initial pool size does not cap
    /// payload length.
    pub fn reserve_large(
        &mut self,
        message: Message,
        payload_len: usize,
    ) -> io::Result<LargeReservation<'_>> {
        self.core.reserve_large(message, payload_len)
    }
    /// Consume a writable slab into a framing builder bound to this writer.
    /// The builder may move to a producer thread and remains valid across rotation.
    /// Seal it before passing its immutable block to [`Self::append_prepared_records`].
    pub fn prepare_records(&self, slab: WritableSlab) -> PreparedRecordsBuilder {
        self.core.prepare_records(slab)
    }
    /// Accept an immutable prepared block as one contiguous file range.
    ///
    /// The block must originate from this writer, including across rotation. On
    /// rejection, [`Rejected`] returns the complete block and error; no file range
    /// is consumed for it. `WouldBlock` permits retry after driving pending I/O.
    /// Other errors include incompatible framing, registration and pending I/O
    /// failures. Earlier accepted writes may progress during a rejected attempt.
    ///
    /// On success, storage retains the backing through completion. Success means
    /// acceptance; [`Self::flush_written`] and [`Self::finish`] expose later failures.
    pub fn append_prepared_records(
        &mut self,
        block: PreparedBlock,
    ) -> Result<crate::file_offsets::FileRange, Rejected<PreparedBlock>> {
        self.update(|c| c.append_prepared_records(block))
    }
    /// Frame and accept an immutable external payload without copying its bytes.
    ///
    /// `message.channel` must be declared; direct-I/O addresses and lengths must
    /// satisfy the writer alignment. Storage adds a managed prefix and retains the
    /// read lease through completion. Rejection returns the intact lease and error
    /// without accepting its file range; earlier writes may still progress. Errors
    /// include invalid metadata/alignment, backpressure, registration and I/O failure.
    /// Drive pending I/O before retrying `WouldBlock`; success means acceptance.
    pub fn append_external_payload(
        &mut self,
        message: Message,
        lease: crate::mapped::ReadLease,
    ) -> Result<crate::file_offsets::FileRange, Rejected<crate::mapped::ReadLease>> {
        self.update(|c| c.append_external_payload(message, lease))
    }
    /// Accept complete external message records and padding with their bytes intact.
    ///
    /// Records must use declared channels; file-level records belong to the writer.
    /// Storage retains the read lease through completion. Malformed framing, invalid
    /// alignment, backpressure, registration or I/O errors return the intact lease
    /// in [`Rejected`] without accepting its range. Earlier writes may progress.
    /// Success means acceptance; explicit flush/finish reports later failures.
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
    /// Optionally refresh completions and flush an aged batch while idle.
    /// Submissions perform this work automatically. Without calls, an idle batch
    /// remains buffered until the next submission, flush, or finish.
    pub fn poll(&mut self, now: Instant) -> io::Result<()> {
        self.update(|c| c.poll(now))
    }
    /// Flush buffered messages and wait for accepted writes to complete.
    /// Returns retained write/submission errors. File synchronization and terminal
    /// MCAP records are performed by [`Self::finish`].
    pub fn flush_written(&mut self) -> io::Result<()> {
        self.update(McapCore::flush_written)
    }
    /// Consume this writer, drain writes, append the terminal MCAP records and sync.
    /// Returns write, footer, logical-length truncation or sync errors. On failure
    /// the writer is consumed and the recording may be incomplete.
    pub fn finish(mut self) -> io::Result<()> {
        self.update(McapCore::seal)
    }
    /// Finalize the current recording and take ownership of `file` for the next one.
    /// Repeats schema/channel records and preserves producer registration caches.
    /// The caller supplies dictionary checkpoints. Errors may occur after the old
    /// file is finalized or the new file is installed; there is no rollback.
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
