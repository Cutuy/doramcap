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
    large: Vec<BufferLease>,
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
fn padded_len(used: usize, alignment: usize) -> usize {
    if used.is_multiple_of(alignment) {
        return used;
    }
    let mut len = used.next_multiple_of(alignment);
    if len - used < 9 {
        len += alignment;
    }
    len
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
            Writer::direct(file, pool.clone(), alignment)?
        } else {
            Writer::new(file, buffers)?
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
            large: Vec::with_capacity(buffers as usize),
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
        if self.pool.available() == 0 {
            self.writer.flush_written()?;
        }
        self.pool.reserve(len).map_err(|_| invalid())
    }
    fn submit(&mut self, lease: BufferLease) -> io::Result<()> {
        if self.writer.outstanding() == self.pool.capacity() {
            self.writer.flush_written()?;
        }
        self.writer.append(lease)?;
        self.writes += 1;
        Ok(())
    }
    /// Dropping without commit abandons the message without reserving file bytes.
    pub fn reserve(
        &mut self,
        message: Message,
        payload_len: usize,
    ) -> io::Result<MessageReservation<'_>> {
        let total = payload_len.checked_add(31).ok_or_else(invalid)?;
        if total > self.record_capacity() || !self.channels.contains(&message.channel) {
            return Err(invalid());
        }
        self.poll(Instant::now())?;
        if self.used + total > self.record_capacity() {
            self.flush_combining()?;
        }
        if self.active.is_none() {
            self.active = Some(self.acquire(self.buffer_size)?);
        }
        let start = self.used;
        let end = start + total;
        let out = &mut self.active.as_mut().unwrap().as_mut_slice()[start..end];
        out[0] = 5;
        out[1..9].copy_from_slice(&((total - 9) as u64).to_le_bytes());
        out[9..11].copy_from_slice(&message.channel.to_le_bytes());
        out[11..15].copy_from_slice(&message.sequence.to_le_bytes());
        out[15..23].copy_from_slice(&message.log_time.to_le_bytes());
        out[23..31].copy_from_slice(&message.publish_time.to_le_bytes());
        Ok(MessageReservation {
            writer: self,
            start,
            end,
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
            self.flush_combining()?;
        }
        self.writer.progress(false)
    }
    fn flush_combining(&mut self) -> io::Result<()> {
        if let Some(mut lease) = self.active.take() {
            let used = self.used;
            self.used = 0;
            self.since = None;
            if used > 0 {
                let physical = padding(lease.as_mut_slice(), used, self.alignment);
                lease.truncate(physical);
                self.submit(lease)?;
            }
        }
        Ok(())
    }
    pub fn flush_written(&mut self) -> io::Result<()> {
        self.flush_combining()?;
        self.writer.flush_written()
    }
    fn seal(&mut self) -> io::Result<()> {
        self.flush_combining()?;
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
    pub fn max_large_payload(&self) -> usize {
        self.pool
            .capacity()
            .saturating_mul(self.buffer_size)
            .saturating_sub(if self.alignment > 1 {
                self.alignment
            } else {
                0
            })
            .saturating_sub(31)
    }
    pub fn reserve_large(
        &mut self,
        message: Message,
        payload_len: usize,
    ) -> io::Result<LargeReservation<'_>> {
        if payload_len > self.max_large_payload()
            || !self.channels.contains(&message.channel)
            || !self.large.is_empty()
        {
            return Err(invalid());
        }
        self.flush_written()?;
        let total = payload_len.checked_add(31).ok_or_else(invalid)?;
        let physical = padded_len(total, self.alignment);
        let mut remaining = physical;
        while remaining > 0 {
            let len = remaining.min(self.buffer_size);
            match self.acquire(len) {
                Ok(lease) => self.large.push(lease),
                Err(e) => {
                    self.large.clear();
                    return Err(e);
                }
            }
            remaining -= len;
        }
        let out = self.large[0].as_mut_slice();
        out[0] = 5;
        out[1..9].copy_from_slice(&((total - 9) as u64).to_le_bytes());
        out[9..11].copy_from_slice(&message.channel.to_le_bytes());
        out[11..15].copy_from_slice(&message.sequence.to_le_bytes());
        out[15..23].copy_from_slice(&message.log_time.to_le_bytes());
        out[23..31].copy_from_slice(&message.publish_time.to_le_bytes());
        if physical > total {
            for index in total..physical {
                self.large[index / self.buffer_size].as_mut_slice()[index % self.buffer_size] = 0;
            }
            let mut header = [0; 9];
            header[0] = 0x80;
            header[1..].copy_from_slice(&((physical - total - 9) as u64).to_le_bytes());
            for (i, byte) in header.into_iter().enumerate() {
                let index = total + i;
                self.large[index / self.buffer_size].as_mut_slice()[index % self.buffer_size] =
                    byte;
            }
        }
        Ok(LargeReservation {
            writer: self,
            payload_len,
            filled: 0,
        })
    }
}
pub struct MessageReservation<'a> {
    writer: &'a mut McapWriter,
    start: usize,
    end: usize,
}
impl MessageReservation<'_> {
    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.writer.active.as_mut().unwrap().as_mut_slice()[self.start + 31..self.end]
    }
    pub fn payload(&self) -> &[u8] {
        &self.writer.active.as_ref().unwrap().as_slice()[self.start + 31..self.end]
    }
    pub fn commit(self) -> io::Result<()> {
        self.writer.used = self.end;
        self.writer.since.get_or_insert_with(Instant::now);
        if self.end == self.writer.record_capacity() {
            self.writer.flush_combining()?;
        }
        Ok(())
    }
}

/// Segmented encoding destination. Protobuf encoders can write through BufMut;
/// bytes fields are copied once into their final payload locations.
pub struct LargeReservation<'a> {
    writer: &'a mut McapWriter,
    payload_len: usize,
    filled: usize,
}
// SAFETY: chunk_mut exposes initialized, exclusively leased bytes; advance_mut
// checks the remaining payload bound and never exposes headers or padding.
unsafe impl bytes::BufMut for LargeReservation<'_> {
    fn remaining_mut(&self) -> usize {
        self.payload_len - self.filled
    }
    unsafe fn advance_mut(&mut self, cnt: usize) {
        assert!(cnt <= self.payload_len - self.filled);
        self.filled += cnt;
    }
    fn chunk_mut(&mut self) -> &mut bytes::buf::UninitSlice {
        if self.filled == self.payload_len {
            return bytes::buf::UninitSlice::new(&mut []);
        }
        let offset = 31 + self.filled;
        let index = offset / self.writer.buffer_size;
        let start = offset % self.writer.buffer_size;
        let end = self.writer.large[index]
            .as_slice()
            .len()
            .min(start + self.payload_len - self.filled);
        bytes::buf::UninitSlice::new(&mut self.writer.large[index].as_mut_slice()[start..end])
    }
}
impl LargeReservation<'_> {
    pub fn payload_len(&self) -> usize {
        self.payload_len
    }
    pub fn copy_payload_to(&self, destination: &mut [u8]) {
        assert_eq!(destination.len(), self.payload_len);
        let mut offset = 0;
        for (index, lease) in self.writer.large.iter().enumerate() {
            let start = if index == 0 { 31 } else { 0 };
            let len = (lease.as_slice().len() - start).min(destination.len() - offset);
            destination[offset..offset + len]
                .copy_from_slice(&lease.as_slice()[start..start + len]);
            offset += len;
        }
    }

    pub fn commit(self) -> io::Result<()> {
        if self.filled != self.payload_len {
            return Err(invalid());
        }
        let mut leases = std::mem::take(&mut self.writer.large);
        let mut result = Ok(());
        for lease in leases.drain(..) {
            if result.is_ok() {
                result = self.writer.submit(lease);
            }
        }
        self.writer.large = leases;
        result
    }
}
impl Drop for LargeReservation<'_> {
    fn drop(&mut self) {
        self.writer.large.clear();
    }
}
