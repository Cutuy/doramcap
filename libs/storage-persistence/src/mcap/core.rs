//! Private recording state, framing, and reservation ownership.
use super::{Channel, McapConfig, Message, Schema};
use crate::{
    managed_buffers::{BufferLease, BufferPool},
    observability::{EventData, EventKind, Ids, McapSnapshot, elapsed_us},
    slabs::{Rejected, SealedSlab, SlabPool, WritableSlab},
    timeline::Timeline,
    writer::Writer,
};
use std::{
    fs::File,
    io,
    sync::Arc,
    time::{Duration, Instant},
};
const MAGIC: &[u8; 8] = b"\x89MCAP0\r\n";
pub(super) struct McapCore {
    combining_timeline: Timeline,
    combining_messages: usize,
    writer: Writer,
    pool: BufferPool,
    prefix_pool: BufferPool,
    channels: Vec<u16>,
    buffer_size: usize,
    active: Option<BufferLease>,
    used: usize,
    since: Option<Instant>,
    max_age: Duration,
    metrics: crate::observability::McapMetrics,
    alignment: usize,
    metadata: Vec<u8>,
    prepared_format: Arc<PreparedFormat>,
}
fn invalid() -> io::Error {
    io::ErrorKind::InvalidInput.into()
}
fn string(out: &mut Vec<u8>, value: &[u8]) -> io::Result<()> {
    let len = u32::try_from(value.len()).map_err(|_| invalid())?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value);
    Ok(())
}
fn record(out: &mut Vec<u8>, op: u8, content: &[u8]) {
    out.push(op);
    out.extend_from_slice(&(content.len() as u64).to_le_bytes());
    out.extend_from_slice(content);
}
fn checked_padded_len(used: usize, alignment: usize) -> Option<usize> {
    if used.is_multiple_of(alignment) {
        return Some(used);
    }
    let len = used.checked_next_multiple_of(alignment)?;
    if len - used < 9 {
        len.checked_add(alignment)
    } else {
        Some(len)
    }
}
fn padded_len(used: usize, alignment: usize) -> usize {
    checked_padded_len(used, alignment).expect("validated record size")
}
fn padding(out: &mut [u8], used: usize, alignment: usize) -> usize {
    let len = padded_len(used, alignment);
    if len > used {
        out[used..len].fill(0);
        out[used] = 0x80;
        out[used + 1..used + 9].copy_from_slice(&((len - used - 9) as u64).to_le_bytes());
    }
    len
}
impl McapCore {
    pub(super) fn publish_metrics(&self) {
        self.metrics
            .publish(|| self.snapshot(), || self.writer.active_writes());
    }

    pub(super) fn new(
        file: File,
        schema: Schema<'_>,
        channels: &[Channel<'_>],
        config: McapConfig,
    ) -> io::Result<Self> {
        let McapConfig {
            buffers,
            buffer_size,
            alignment,
            io: config,
            observer,
            mut metrics,
        } = config;
        metrics.reserve(config.pending_capacity)?;
        if !matches!(alignment, 1 | 512 | 4096)
            || (alignment > 1
                && (buffer_size < 2 * alignment || !buffer_size.is_multiple_of(alignment)))
        {
            return Err(invalid());
        }
        if buffer_size < 50 || channels.is_empty() {
            return Err(invalid());
        }
        let mut ids = Vec::with_capacity(channels.len());
        for channel in channels {
            if ids.contains(&channel.id) {
                return Err(invalid());
            }
            ids.push(channel.id);
        }
        let pool = BufferPool::new(buffers as usize, buffer_size, 4096).map_err(|_| invalid())?;
        let writer = if alignment > 1 {
            Writer::direct_with_observer(file, pool.clone(), alignment, config, observer)?
        } else {
            Writer::with_observer(file, config, observer)?
        };
        let mut this = Self {
            combining_timeline: Timeline::default(),
            combining_messages: 0,
            writer,
            prefix_pool: BufferPool::new(buffers as usize, 4096, 4096).map_err(|_| invalid())?,
            pool,
            prepared_format: Arc::new(PreparedFormat {
                channels: ids.clone(),
                alignment,
            }),
            channels: ids,
            buffer_size,
            active: None,
            used: 0,
            since: None,
            max_age: Duration::from_millis(100),
            metrics,
            alignment,
            metadata: Vec::new(),
        };
        let mut bytes = MAGIC.to_vec();
        let mut content = Vec::new();
        string(&mut content, b"")?;
        string(&mut content, b"doramcap")?;
        record(&mut bytes, 1, &content);
        content.clear();
        content.extend_from_slice(&1_u16.to_le_bytes());
        string(&mut content, schema.name.as_bytes())?;
        string(&mut content, schema.encoding.as_bytes())?;
        string(&mut content, schema.data)?;
        record(&mut bytes, 3, &content);
        for channel in channels {
            content.clear();
            content.extend_from_slice(&channel.id.to_le_bytes());
            content.extend_from_slice(&1_u16.to_le_bytes());
            string(&mut content, channel.topic.as_bytes())?;
            string(&mut content, channel.encoding.as_bytes())?;
            content.extend_from_slice(&0_u32.to_le_bytes());
            record(&mut bytes, 4, &content);
        }
        if alignment > 1 {
            let used = bytes.len();
            bytes.resize(padded_len(used, alignment), 0);
            padding(&mut bytes, used, alignment);
        }
        for chunk in bytes.chunks(buffer_size) {
            let mut lease = this.acquire(chunk.len())?;
            lease.as_mut_slice().copy_from_slice(chunk);
            this.submit(lease)?;
        }
        this.metadata = bytes;
        this.publish_metrics();
        Ok(this)
    }
    fn acquire(&mut self, len: usize) -> io::Result<BufferLease> {
        let lease = self.pool.reserve(len)?;
        self.writer.prepare(&lease)?;
        Ok(lease)
    }
    fn submit(&mut self, lease: BufferLease) -> io::Result<()> {
        self.writer.append(lease)?;
        Ok(())
    }
    /// Retire registrations and release retained memory only while fully idle.
    pub(super) fn trim(&mut self) -> io::Result<usize> {
        if self.active.is_some() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.writer.trim(&self.pool)
    }
    fn snapshot(&self) -> McapSnapshot {
        McapSnapshot {
            writer: self.writer.snapshot(),
            combining_bytes: self.used,
            combining_messages: self.combining_messages,
            combining_capacity: self.record_capacity(),
            combining_age_us: elapsed_us(self.since),
            managed_available: self.pool.available(),
            managed_capacity: self.pool.capacity(),
            managed_allocated_bytes: self.pool.allocated_bytes(),
            managed_free_bytes: self.pool.free_bytes(),
        }
    }

    /// Register producer-owned slabs before capture starts. Repeated calls are cache hits.
    pub(super) fn prepare_slabs(&mut self, pool: &SlabPool) -> io::Result<()> {
        self.writer.prepare_slabs(pool)
    }
    /// Retire producer registrations after pending writes drain. Readers may remain.
    pub(super) fn retire_slabs(&mut self, pool: &SlabPool) -> io::Result<()> {
        self.writer.retire_slabs(pool)
    }

    pub(super) fn prepare_region(&mut self, region: &crate::mapped::MmapRegion) -> io::Result<()> {
        self.writer.prepare_region(region)
    }
    pub(super) fn retire_region(&mut self, region: &crate::mapped::MmapRegion) -> io::Result<()> {
        self.writer.retire_region(region)
    }
    /// Append an existing immutable payload. Only the managed prefix is written by storage.
    /// Source address and transfer length must satisfy the configured direct-I/O alignment.
    pub(super) fn append_external_payload(
        &mut self,
        message: Message,
        lease: crate::mapped::ReadLease,
    ) -> Result<crate::file_offsets::FileRange, Rejected<crate::mapped::ReadLease>> {
        let prepare = (|| {
            if !self.channels.contains(&message.channel) {
                return Err(invalid());
            }
            let total = lease.as_slice().len().checked_add(31).ok_or_else(invalid)?;
            self.writer.check_capacity(1)?;
            self.writer.prepare_mapped(&lease)?;
            let mut prefix = self
                .prefix_pool
                .reserve(4096)
                .map_err(|_| io::Error::from(io::ErrorKind::WouldBlock))?;
            let bytes = prefix.as_mut_slice();
            bytes.fill(0);
            let header = bytes.len() - 31;
            bytes[0] = 0x80;
            bytes[1..9].copy_from_slice(&((header - 9) as u64).to_le_bytes());
            message_header(&mut bytes[header..], message, total);
            Ok(prefix)
        })();
        let prefix = match prepare {
            Ok(prefix) => prefix,
            Err(error) => {
                self.writer
                    .reject(error.kind(), lease.ids(), lease.as_slice().len() as u64);
                return Err(Rejected {
                    error,
                    value: lease,
                });
            }
        };
        let range = self.writer.try_append_prefixed(prefix, lease)?;
        Ok(range)
    }
    /// Accept caller-stamped message records and optional alignment padding unchanged.
    /// Channels refer to this writer's declared schema/channel table. File-level records
    /// (headers, schemas, channels, footers and indexes) are owned by the writer.
    pub(super) fn append_external_records(
        &mut self,
        lease: crate::mapped::ReadLease,
    ) -> Result<crate::file_offsets::FileRange, Rejected<crate::mapped::ReadLease>> {
        if let Err(error) = self.validate_external_records(lease.as_slice()) {
            self.writer
                .reject(error.kind(), lease.ids(), lease.as_slice().len() as u64);
            return Err(Rejected {
                error,
                value: lease,
            });
        }
        let range = self.writer.try_append_mapped(lease)?;
        Ok(range)
    }
    fn validate_external_records(&self, bytes: &[u8]) -> io::Result<()> {
        let mut position = 0;
        let mut messages = 0;
        while position < bytes.len() {
            let head = bytes
                .get(position..position.checked_add(9).ok_or_else(invalid)?)
                .ok_or_else(invalid)?;
            let body = usize::try_from(u64::from_le_bytes(head[1..9].try_into().unwrap()))
                .map_err(|_| invalid())?;
            let end = position
                .checked_add(9)
                .and_then(|p| p.checked_add(body))
                .ok_or_else(invalid)?;
            let record = bytes.get(position..end).ok_or_else(invalid)?;
            match head[0] {
                5 if body >= 22 => {
                    let channel = u16::from_le_bytes(record[9..11].try_into().unwrap());
                    if !self.channels.contains(&channel) {
                        return Err(invalid());
                    }
                    messages += 1;
                }
                0x80 => {}
                _ => return Err(invalid()),
            }
            position = end;
        }
        if messages == 0 {
            return Err(invalid());
        }
        Ok(())
    }
    /// Obtain a movable framing builder bound to this stream's schema and channels.
    /// It can be sent to a producer thread and remains compatible across rotation.
    pub(super) fn prepare_records(&self, slab: WritableSlab) -> PreparedRecordsBuilder {
        PreparedRecordsBuilder {
            slab,
            format: self.prepared_format.clone(),
            used: 0,
        }
    }
    /// Admit a complete immutable block, preserving its bytes in one contiguous file range.
    /// Blocks from another writer are rejected intact, even if their channel IDs match.
    pub(super) fn append_prepared_records(
        &mut self,
        block: PreparedBlock,
    ) -> Result<crate::file_offsets::FileRange, Rejected<PreparedBlock>> {
        let validation = (|| {
            if !Arc::ptr_eq(&block.format, &self.prepared_format) {
                return Err(invalid());
            }
            self.writer.check_capacity(1)?;
            self.writer.prepare_slab(&block.slab)
        })();
        if let Err(error) = validation {
            self.writer.reject(
                error.kind(),
                block.slab.observation_ids(),
                block.slab.as_slice().len() as u64,
            );
            return Err(Rejected {
                error,
                value: block,
            });
        }
        let PreparedBlock { slab, format } = block;
        match self.writer.try_append_slab(slab) {
            Ok(range) => Ok(range),
            Err(e) => Err(Rejected {
                error: e.error,
                value: PreparedBlock {
                    slab: e.value,
                    format,
                },
            }),
        }
    }

    pub(super) fn reserve(
        &mut self,
        message: Message,
        payload_len: usize,
    ) -> io::Result<MessageReservation<'_>> {
        let result = self.reserve_parts(message, payload_len);
        self.publish_metrics();
        let (start, end, standalone) = result?;
        Ok(MessageReservation {
            writer: self,
            start,
            end,
            standalone,
            committed: false,
        })
    }
    /// Dropping without commit abandons the message without reserving file bytes.
    fn reserve_parts(
        &mut self,
        message: Message,
        payload_len: usize,
    ) -> io::Result<(usize, usize, Option<BufferLease>)> {
        if payload_len > self.max_large_payload() || !self.channels.contains(&message.channel) {
            return Err(invalid());
        }
        let total = payload_len.checked_add(31).ok_or_else(invalid)?;
        if total > self.record_capacity() {
            // Keep any preceding batch until commit. Pending-queue pressure must not
            // prevent acquiring a new contiguous payload allocation.
            let physical = checked_padded_len(total, self.alignment).ok_or_else(invalid)?;
            let mut lease = self.acquire(physical)?;
            message_header(lease.as_mut_slice(), message, total);
            padding(lease.as_mut_slice(), total, self.alignment);
            return Ok((0, total, Some(lease)));
        }
        if self.used + total > self.record_capacity() {
            self.flush_combining()?;
        }
        if self.active.is_none() {
            self.active = Some(self.acquire(self.buffer_size)?);
        }
        let start = self.used;
        let end = start + total;
        message_header(
            &mut self.active.as_mut().unwrap().as_mut_slice()[start..end],
            message,
            total,
        );
        Ok((start, end, None))
    }
    pub(super) fn max_inline_payload(&self) -> usize {
        self.record_capacity().saturating_sub(31)
    }
    fn record_capacity(&self) -> usize {
        self.buffer_size
            - if self.alignment > 1 {
                self.alignment
            } else {
                0
            }
    }
    pub(super) fn next_flush_deadline(&self) -> Option<Instant> {
        self.since.and_then(|t| t.checked_add(self.max_age))
    }
    /// The owner event loop calls this on its timer, including when inputs are idle.
    pub(super) fn poll(&mut self, now: Instant) -> io::Result<()> {
        if self
            .next_flush_deadline()
            .is_some_and(|deadline| now >= deadline)
        {
            // A full pending queue retains the batch. Still drive CQEs so a later
            // poll can admit it; otherwise timer polling could never clear pressure.
            if let Err(error) = self.flush_combining()
                && error.kind() != io::ErrorKind::WouldBlock
            {
                return Err(error);
            }
        }
        self.writer.progress(false)
    }
    fn flush_combining(&mut self) -> io::Result<()> {
        if self.used == 0 {
            self.active = None;
            self.since = None;
            return Ok(());
        }
        self.writer.check_capacity(1)?;
        if let Some(mut lease) = self.active.take() {
            let physical = padding(lease.as_mut_slice(), self.used, self.alignment);
            // Keep the full combining lease on failure so the batch can be retried.
            lease.truncate(physical);
            match self
                .writer
                .try_append_tagged(lease, "combined", &self.combining_timeline)
            {
                Ok(_) => {
                    self.writer
                        .observer()
                        .record(EventKind::CombiningFlushed, || EventData {
                            ids: Ids {
                                writer: self.writer.writer_id(),
                                file: self.writer.file_id(),
                                ..Default::default()
                            },
                            len: physical as u64,
                            duration_us: elapsed_us(self.since),
                            ..Default::default()
                        });
                    self.combining_timeline = Timeline::default();
                    self.combining_messages = 0;
                    self.used = 0;
                    self.since = None;
                }
                Err(mut e) => {
                    e.lease.restore_len(self.buffer_size);
                    self.active = Some(e.lease);
                    return Err(e.error);
                }
            }
        }
        Ok(())
    }
    pub(super) fn flush_written(&mut self) -> io::Result<()> {
        // Explicit flush may wait to admit the last combining buffer.
        if self.used > 0 && self.writer.check_capacity(1).is_err() {
            self.writer.flush_written()?;
        }
        self.flush_combining()?;
        self.writer.flush_written()
    }
    pub(super) fn seal(&mut self) -> io::Result<()> {
        self.flush_written()?;
        let logical_len = self
            .writer
            .accepted_bytes()
            .checked_add(50)
            .ok_or_else(invalid)?;
        let mut lease = self.acquire(if self.alignment > 1 {
            self.alignment
        } else {
            50
        })?;
        let tail = lease.as_mut_slice();
        tail.fill(0);
        tail[0] = 0x0f;
        tail[1..9].copy_from_slice(&4_u64.to_le_bytes());
        tail[13] = 2;
        tail[14..22].copy_from_slice(&20_u64.to_le_bytes());
        tail[42..50].copy_from_slice(MAGIC);
        self.submit(lease)?;
        self.writer.finalize_at(logical_len)
    }

    /// Rotate at a message boundary. The application writes its dictionary
    /// checkpoint after this call; schemas/channels are repeated automatically.
    pub(super) fn rotate(&mut self, file: File) -> io::Result<()> {
        self.seal()?;
        self.writer.replace_file(file)?;
        for start in (0..self.metadata.len()).step_by(self.buffer_size) {
            let end = (start + self.buffer_size).min(self.metadata.len());
            let mut lease = self.acquire(end - start)?;
            lease
                .as_mut_slice()
                .copy_from_slice(&self.metadata[start..end]);
            self.submit(lease)?;
        }
        Ok(())
    }
    /// Physical bytes accepted plus the current combining buffer. Rotation
    /// targets are soft limits: a complete record may exceed the target.
    pub(super) fn file_bytes(&self) -> u64 {
        self.writer.accepted_bytes() + self.used as u64
    }
    /// Address-space bound; independent of the initial pool size.
    pub(super) fn max_large_payload(&self) -> usize {
        (isize::MAX as usize)
            .saturating_sub(self.alignment * 2)
            .saturating_sub(31)
    }
    /// Compatibility BufMut adapter over a single contiguous reservation.
    pub(super) fn reserve_large(
        &mut self,
        message: Message,
        payload_len: usize,
    ) -> io::Result<LargeReservation<'_>> {
        Ok(LargeReservation {
            inner: self.reserve(message, payload_len)?,
            payload_len,
            filled: 0,
        })
    }
}

#[derive(Debug)]
struct PreparedFormat {
    channels: Vec<u16>,
    alignment: usize,
}

/// Exclusive producer-side MCAP framing. Encoders write directly into each payload.
#[derive(Debug)]
pub struct PreparedRecordsBuilder {
    slab: WritableSlab,
    format: Arc<PreparedFormat>,
    used: usize,
}
impl PreparedRecordsBuilder {
    /// Reserve a complete message. Returned payload bytes start zeroed; encode before
    /// requesting another message. Admission failures leave previous messages intact.
    pub fn reserve(&mut self, message: Message, payload_len: usize) -> io::Result<&mut [u8]> {
        if !self.format.channels.contains(&message.channel) {
            return Err(invalid());
        }
        let total = payload_len.checked_add(31).ok_or_else(invalid)?;
        let end = self.used.checked_add(total).ok_or_else(invalid)?;
        let physical = checked_padded_len(end, self.format.alignment).ok_or_else(invalid)?;
        if physical > self.slab.capacity() {
            return Err(io::ErrorKind::WriteZero.into());
        }
        let start = self.used;
        let out = &mut self.slab.bytes_mut()[start..end];
        message_header(out, message, total);
        out[31..].fill(0);
        self.used = end;
        Ok(&mut out[31..])
    }
    /// Consume exclusive access after writing valid alignment padding.
    pub fn seal(mut self) -> Result<PreparedBlock, Rejected<Self>> {
        if self.used == 0 {
            return Err(Rejected {
                error: invalid(),
                value: self,
            });
        }
        let len = padding(self.slab.bytes_mut(), self.used, self.format.alignment);
        // reserve() established nonempty initialized bounds, including padding.
        let slab = self
            .slab
            .seal(len)
            .expect("validated prepared block length");
        Ok(PreparedBlock {
            slab,
            format: self.format,
        })
    }
}
/// Complete MCAP message records and padding, sealed for this writer's format.
#[derive(Clone, Debug)]
pub struct PreparedBlock {
    slab: SealedSlab,
    format: Arc<PreparedFormat>,
}
impl PreparedBlock {
    /// Share the entire immutable slab with a reader. Storage uses this same range.
    pub fn slab(&self) -> &SealedSlab {
        &self.slab
    }
}

fn message_header(out: &mut [u8], message: Message, total: usize) {
    out[0] = 5;
    out[1..9].copy_from_slice(&((total - 9) as u64).to_le_bytes());
    out[9..11].copy_from_slice(&message.channel.to_le_bytes());
    out[11..15].copy_from_slice(&message.sequence.to_le_bytes());
    out[15..23].copy_from_slice(&message.log_time.to_le_bytes());
    out[23..31].copy_from_slice(&message.publish_time.to_le_bytes());
}

pub struct MessageReservation<'a> {
    writer: &'a mut McapCore,
    start: usize,
    end: usize,
    standalone: Option<BufferLease>,
    committed: bool,
}
impl MessageReservation<'_> {
    pub fn payload_mut(&mut self) -> &mut [u8] {
        assert!(!self.committed, "reservation already committed");
        let lease = self
            .standalone
            .as_mut()
            .or(self.writer.active.as_mut())
            .unwrap();
        &mut lease.as_mut_slice()[self.start + 31..self.end]
    }
    pub fn payload(&self) -> &[u8] {
        assert!(!self.committed, "reservation already committed");
        let lease = self
            .standalone
            .as_ref()
            .or(self.writer.active.as_ref())
            .unwrap();
        &lease.as_slice()[self.start + 31..self.end]
    }
    /// Retryable commit: failure leaves this reservation and its bytes intact.
    pub fn try_commit(&mut self) -> io::Result<()> {
        let result = self.commit_inner();
        self.writer.publish_metrics();
        result
    }
    fn commit_inner(&mut self) -> io::Result<()> {
        if self.committed {
            return Err(invalid());
        }
        if let Some(lease) = self.standalone.take() {
            match self
                .writer
                .writer
                .try_append_tagged(lease, "large", &Timeline::default())
            {
                Ok(_) => {}
                Err(e) => {
                    self.standalone = Some(e.lease);
                    return Err(e.error);
                }
            }
        } else {
            if self.writer.used == 0 {
                self.writer.combining_timeline = Timeline::combining(
                    self.writer.writer.writer_id(),
                    self.writer.writer.file_id(),
                );
            }
            self.writer.used = self.end;
            self.writer.combining_messages += 1;
            self.writer.since.get_or_insert_with(Instant::now);
        }
        self.committed = true;
        Ok(())
    }
    /// Drive pending writes while retaining a reservation rejected by backpressure.
    pub fn progress(&mut self, wait: bool) -> io::Result<()> {
        let result = self.writer.writer.progress(wait);
        self.writer.publish_metrics();
        result
    }
    /// Compatibility consuming commit; use try_commit to retain ownership on failure.
    pub fn commit(mut self) -> io::Result<()> {
        self.try_commit()
    }
}

impl Drop for MessageReservation<'_> {
    fn drop(&mut self) {
        drop(self.standalone.take());
        self.writer.publish_metrics();
    }
}

pub struct LargeReservation<'a> {
    inner: MessageReservation<'a>,
    payload_len: usize,
    filled: usize,
}
// SAFETY: exposes initialized, exclusively leased payload bytes; advancement is bounded.
unsafe impl bytes::BufMut for LargeReservation<'_> {
    fn remaining_mut(&self) -> usize {
        self.payload_len - self.filled
    }
    unsafe fn advance_mut(&mut self, cnt: usize) {
        assert!(cnt <= self.payload_len - self.filled);
        self.filled += cnt;
    }
    fn chunk_mut(&mut self) -> &mut bytes::buf::UninitSlice {
        bytes::buf::UninitSlice::new(&mut self.inner.payload_mut()[self.filled..])
    }
}
impl LargeReservation<'_> {
    pub fn payload_len(&self) -> usize {
        self.payload_len
    }
    pub fn payload(&self) -> &[u8] {
        self.inner.payload()
    }
    pub fn payload_mut(&mut self) -> &mut [u8] {
        self.filled = self.payload_len;
        self.inner.payload_mut()
    }
    pub fn copy_payload_to(&self, destination: &mut [u8]) {
        destination.copy_from_slice(self.inner.payload());
    }
    pub fn try_commit(&mut self) -> io::Result<()> {
        if self.filled != self.payload_len {
            return Err(invalid());
        }
        self.inner.try_commit()
    }
    pub fn progress(&mut self, wait: bool) -> io::Result<()> {
        self.inner.progress(wait)
    }
    pub fn commit(mut self) -> io::Result<()> {
        self.try_commit()
    }
}
