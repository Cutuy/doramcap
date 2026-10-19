//! Uncompressed, index-free MCAP framing. Payload encoding belongs to the caller.
use crate::{
    managed_buffers::{BufferLease, BufferPool},
    writer::Writer,
};
use std::{
    fs::File,
    io,
    time::{Duration, Instant},
};
const MAGIC: &[u8; 8] = b"\x89MCAP0\r\n";
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
/// Point-in-time owner-thread diagnostics. Byte completion totals can contain holes.
#[cfg(feature = "diagnostics")]
#[derive(Clone, Copy, Debug)]
pub struct Diagnostics {
    pub combining_bytes: usize,
    pub combining_capacity: usize,
    pub combining_age_us: u64,
    pub buffers_available: usize,
    pub buffers_total: usize,
    pub allocated_bytes: usize,
    pub free_bytes: usize,
    pub in_flight_bytes: usize,
    pub registered_bytes: usize,
    pub pending_capacity: usize,
    pub outstanding: usize,
    pub accepted_bytes: u64,
    pub completed_bytes: u64,
    pub submitted_operations: u64,
    pub completions: u64,
}
pub struct McapWriter {
    writer: Writer,
    pool: BufferPool,
    channels: Vec<u16>,
    buffer_size: usize,
    active: Option<BufferLease>,
    used: usize,
    since: Option<Instant>,
    max_age: Duration,
    writes: u64,
    alignment: usize,
    metadata: Vec<u8>,
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
impl McapWriter {
    pub fn new(
        file: File,
        buffers: u32,
        buffer_size: usize,
        schema: Schema<'_>,
        channels: &[Channel<'_>],
    ) -> io::Result<Self> {
        Self::build(file, buffers, buffer_size, schema, channels, 1)
    }
    /// File must be opened with O_DIRECT. Allocation sizes are alignment multiples.
    pub fn direct(
        file: File,
        buffers: u32,
        buffer_size: usize,
        schema: Schema<'_>,
        channels: &[Channel<'_>],
        alignment: usize,
    ) -> io::Result<Self> {
        if !matches!(alignment, 512 | 4096)
            || buffer_size < 2 * alignment
            || !buffer_size.is_multiple_of(alignment)
        {
            return Err(invalid());
        }
        Self::build(file, buffers, buffer_size, schema, channels, alignment)
    }
    fn build(
        file: File,
        buffers: u32,
        buffer_size: usize,
        schema: Schema<'_>,
        channels: &[Channel<'_>],
        alignment: usize,
    ) -> io::Result<Self> {
        Self::with_config(
            file,
            buffers,
            buffer_size,
            schema,
            channels,
            alignment,
            crate::writer::WriterConfig::default(),
        )
    }
    /// Configure I/O admission independently of the initial allocation count.
    pub fn with_config(
        file: File,
        buffers: u32,
        buffer_size: usize,
        schema: Schema<'_>,
        channels: &[Channel<'_>],
        alignment: usize,
        config: crate::writer::WriterConfig,
    ) -> io::Result<Self> {
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
            Writer::direct_with_config(file, pool.clone(), alignment, config)?
        } else {
            Writer::with_config(file, config)?
        };
        let mut this = Self {
            writer,
            pool,
            channels: ids,
            buffer_size,
            active: None,
            used: 0,
            since: None,
            max_age: Duration::from_millis(100),
            writes: 0,
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
        Ok(this)
    }
    fn acquire(&mut self, len: usize) -> io::Result<BufferLease> {
        let lease = self.pool.reserve(len)?;
        self.writer.prepare(&lease)?;
        Ok(lease)
    }
    fn submit(&mut self, lease: BufferLease) -> io::Result<()> {
        self.writer.append(lease)?;
        self.writes += 1;
        Ok(())
    }
    /// Retire registrations and release retained memory only while fully idle.
    pub fn trim(&mut self) -> io::Result<usize> {
        if self.active.is_some() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.writer.trim(&self.pool)
    }
    pub fn allocated_bytes(&self) -> usize {
        self.pool.allocated_bytes()
    }
    pub fn registered_bytes(&self) -> usize {
        self.writer.registered_bytes()
    }
    pub fn pending_writes(&self) -> usize {
        self.writer.outstanding()
    }
    /// Dropping without commit abandons the message without reserving file bytes.
    pub fn reserve(
        &mut self,
        message: Message,
        payload_len: usize,
    ) -> io::Result<MessageReservation<'_>> {
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
            return Ok(MessageReservation {
                writer: self,
                start: 0,
                end: total,
                standalone: Some(lease),
                committed: false,
            });
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
        Ok(MessageReservation {
            writer: self,
            start,
            end,
            standalone: None,
            committed: false,
        })
    }
    pub fn max_inline_payload(&self) -> usize {
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
    #[cfg(feature = "diagnostics")]
    pub fn diagnostics(&self) -> Diagnostics {
        Diagnostics {
            combining_bytes: self.used,
            combining_capacity: self.record_capacity(),
            combining_age_us: self
                .since
                .map_or(0, |t| t.elapsed().as_micros().min(u64::MAX as u128) as u64),
            buffers_available: self.pool.available(),
            buffers_total: self.pool.capacity(),
            allocated_bytes: self.pool.allocated_bytes(),
            free_bytes: self.pool.free_bytes(),
            in_flight_bytes: self.writer.in_flight_bytes(),
            registered_bytes: self.writer.registered_bytes(),
            pending_capacity: self.writer.pending_capacity(),
            outstanding: self.writer.outstanding(),
            accepted_bytes: self.writer.accepted_bytes(),
            completed_bytes: self.writer.written_bytes(),
            submitted_operations: self.writer.submitted_operations(),
            completions: self.writer.completion_count(),
        }
    }
    #[cfg(feature = "diagnostics")]
    pub fn write_events(&self) -> impl Iterator<Item = crate::writer::WriteEvent> + '_ {
        self.writer.events()
    }
    pub fn storage_writes(&self) -> u64 {
        self.writes
    }
    pub fn next_flush_deadline(&self) -> Option<Instant> {
        self.since.and_then(|t| t.checked_add(self.max_age))
    }
    /// The owner event loop calls this on its timer, including when inputs are idle.
    pub fn poll(&mut self, now: Instant) -> io::Result<()> {
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
            match self.writer.try_append(lease) {
                Ok(_) => {
                    self.writes += 1;
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
    pub fn flush_written(&mut self) -> io::Result<()> {
        // Explicit flush may wait to admit the last combining buffer.
        if self.used > 0 && self.writer.check_capacity(1).is_err() {
            self.writer.flush_written()?;
        }
        self.flush_combining()?;
        self.writer.flush_written()
    }
    fn seal(&mut self) -> io::Result<()> {
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
    pub fn finish(mut self) -> io::Result<()> {
        self.seal()
    }
    /// Seal and sync, returning final counters after the footer write retires.
    #[cfg(feature = "diagnostics")]
    pub fn finish_with_diagnostics(mut self) -> io::Result<Diagnostics> {
        self.seal()?;
        Ok(self.diagnostics())
    }
    /// Rotate at a message boundary. The application writes its dictionary
    /// checkpoint after this call; schemas/channels are repeated automatically.
    pub fn rotate(&mut self, file: File) -> io::Result<()> {
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
    pub fn file_bytes(&self) -> u64 {
        self.writer.accepted_bytes() + self.used as u64
    }
    /// Address-space bound; independent of the initial pool size.
    pub fn max_large_payload(&self) -> usize {
        (isize::MAX as usize)
            .saturating_sub(self.alignment * 2)
            .saturating_sub(31)
    }
    /// Compatibility BufMut adapter over a single contiguous reservation.
    pub fn reserve_large(
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

fn message_header(out: &mut [u8], message: Message, total: usize) {
    out[0] = 5;
    out[1..9].copy_from_slice(&((total - 9) as u64).to_le_bytes());
    out[9..11].copy_from_slice(&message.channel.to_le_bytes());
    out[11..15].copy_from_slice(&message.sequence.to_le_bytes());
    out[15..23].copy_from_slice(&message.log_time.to_le_bytes());
    out[23..31].copy_from_slice(&message.publish_time.to_le_bytes());
}

pub struct MessageReservation<'a> {
    writer: &'a mut McapWriter,
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
        if self.committed {
            return Err(invalid());
        }
        if self.standalone.is_some() {
            // Earlier committed small records retain their file order. This may queue
            // that batch even if this reservation then needs to retry its admission.
            self.writer.flush_combining()?;
        }
        if let Some(lease) = self.standalone.take() {
            match self.writer.writer.try_append(lease) {
                Ok(_) => {
                    self.writer.writes += 1;
                }
                Err(e) => {
                    self.standalone = Some(e.lease);
                    return Err(e.error);
                }
            }
        } else {
            self.writer.used = self.end;
            self.writer.since.get_or_insert_with(Instant::now);
        }
        self.committed = true;
        Ok(())
    }
    /// Drive pending writes while retaining a reservation rejected by backpressure.
    pub fn progress(&mut self, wait: bool) -> io::Result<()> {
        self.writer.writer.progress(wait)
    }
    /// Compatibility consuming commit; use try_commit to retain ownership on failure.
    pub fn commit(mut self) -> io::Result<()> {
        self.try_commit()
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
