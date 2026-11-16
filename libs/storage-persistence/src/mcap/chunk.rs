//! Uncompressed chunk framing and caller-supplied slab validation.
use super::core::{ALIGNMENT, checked_padded_len, invalid, record};
use std::io;

pub(super) const HEADER: usize = 49;
const INDEX_HEADER: usize = 15;
const INDEX_ENTRY: usize = 16;

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn channel(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes[9..11].try_into().unwrap())
}
fn next(bytes: &[u8], offset: usize) -> io::Result<&[u8]> {
    let head = bytes
        .get(offset..)
        .filter(|b| b.len() >= 9)
        .ok_or_else(invalid)?;
    let len = usize::try_from(u64_at(head, 1)).map_err(|_| invalid())?;
    head.get(..len.checked_add(9).ok_or_else(invalid)?)
        .ok_or_else(invalid)
}
fn messages(
    bytes: &[u8],
    mut visit: impl FnMut(usize, &[u8]) -> io::Result<()>,
) -> io::Result<(u64, u64, usize)> {
    let (mut min, mut max, mut count, mut offset) = (u64::MAX, 0, 0, 0);
    while offset < bytes.len() {
        let msg = next(bytes, offset)?;
        if msg[0] != 5 || msg.len() < 31 {
            return Err(invalid());
        }
        let time = u64_at(msg, 15);
        min = min.min(time);
        max = max.max(time);
        count += 1;
        visit(offset, msg)?;
        offset += msg.len();
    }
    if count == 0 {
        return Err(invalid());
    }
    Ok((min, max, count))
}
fn pad(out: &mut [u8], start: usize) {
    if start < out.len() {
        let body = (out.len() - start - 9) as u64;
        out[start..].fill(0);
        out[start] = 0x80;
        out[start + 1..start + 9].copy_from_slice(&body.to_le_bytes());
    }
}

/// Finalize in the extent chosen solely from mandatory framing and message bytes.
pub(super) fn frame(out: &mut [u8], end: usize) -> io::Result<usize> {
    if end < HEADER || end > out.len() {
        return Err(invalid());
    }
    let physical = checked_padded_len(end)
        .filter(|n| *n <= out.len())
        .ok_or_else(invalid)?;
    let out = &mut out[..physical];
    let (chunk, tail) = out.split_at_mut(end);
    let records = &chunk[HEADER..];
    let (min, max, count) = messages(records, |_, _| Ok(()))?;
    let raw_len = records.len() as u64;
    // Fixed header is written directly; no payload is read or moved.
    chunk[..HEADER].fill(0);
    chunk[0] = 6;
    chunk[1..9].copy_from_slice(&(40 + raw_len).to_le_bytes());
    chunk[9..17].copy_from_slice(&min.to_le_bytes());
    chunk[17..25].copy_from_slice(&max.to_le_bytes());
    chunk[25..33].copy_from_slice(&raw_len.to_le_bytes());
    chunk[41..49].copy_from_slice(&raw_len.to_le_bytes());
    let records = &chunk[HEADER..];
    let mut used = 0;
    // A necessary lower bound from the actual completed batch avoids repeated scans
    // when even a single channel's entries cannot fit in the remaining bytes.
    if count
        .checked_mul(INDEX_ENTRY)
        .and_then(|n| n.checked_add(INDEX_HEADER))
        .is_some_and(|n| n <= tail.len())
    {
        let attempt = messages(records, |_, msg| {
            let id = channel(msg);
            let mut offset = 0;
            while offset < used {
                let index = next(&tail[..used], offset)?;
                if channel(index) == id {
                    return Ok(());
                }
                offset += index.len();
            }
            let start = used;
            used = used
                .checked_add(INDEX_HEADER)
                .filter(|n| *n <= tail.len())
                .ok_or_else(invalid)?;
            messages(records, |offset, msg| {
                if channel(msg) == id {
                    let end = used
                        .checked_add(INDEX_ENTRY)
                        .filter(|n| *n <= tail.len())
                        .ok_or_else(invalid)?;
                    tail[used..used + 8].copy_from_slice(&u64_at(msg, 15).to_le_bytes());
                    tail[used + 8..end].copy_from_slice(&(offset as u64).to_le_bytes());
                    used = end;
                }
                Ok(())
            })?;
            tail[start] = 7;
            tail[start + 1..start + 9].copy_from_slice(&((used - start - 9) as u64).to_le_bytes());
            tail[start + 9..start + 11].copy_from_slice(&id.to_le_bytes());
            tail[start + 11..start + 15]
                .copy_from_slice(&((used - start - INDEX_HEADER) as u32).to_le_bytes());
            Ok(())
        });
        if attempt.is_err() || (1..9).contains(&(tail.len() - used)) {
            used = 0;
        }
    }
    pad(tail, used);
    Ok(physical)
}

/// One reusable cursor per declared channel; slab validation skips payload bytes.
pub(super) struct Cursor {
    channel: u16,
    next: usize,
    end: usize,
}
pub(super) fn cursors(channels: &[u16]) -> io::Result<Vec<Cursor>> {
    let mut cursors = Vec::new();
    cursors
        .try_reserve_exact(channels.len())
        .map_err(io::Error::other)?;
    cursors.extend(channels.iter().map(|&channel| Cursor {
        channel,
        next: 0,
        end: 0,
    }));
    cursors.sort_unstable_by_key(|cursor| cursor.channel);
    Ok(cursors)
}

pub(super) struct View<'a> {
    pub bytes: &'a [u8],
    pub end: usize,
    pub index_end: usize,
}
impl<'a> View<'a> {
    /// Accept one uncompressed, CRC-omitted chunk, complete indexes in record order,
    /// and optional private padding. Payload contents remain opaque.
    pub fn parse(bytes: &'a [u8], channels: &mut [Cursor]) -> io::Result<Self> {
        if bytes.is_empty() || !bytes.len().is_multiple_of(ALIGNMENT) {
            return Err(invalid());
        }
        let chunk = next(bytes, 0)?;
        if chunk[0] != 6 || chunk.len() < HEADER || u32_at(chunk, 33) != 0 || u32_at(chunk, 37) != 0
        {
            return Err(invalid());
        }
        let raw = (chunk.len() - HEADER) as u64;
        if u64_at(chunk, 25) != raw || u64_at(chunk, 41) != raw {
            return Err(invalid());
        }
        for cursor in channels.iter_mut() {
            cursor.next = 0;
            cursor.end = 0;
        }
        let mut offset = chunk.len();
        while offset < bytes.len() && bytes[offset] == 7 {
            let index = next(bytes, offset)?;
            if index.len() <= INDEX_HEADER
                || u32_at(index, 11) as usize != index.len() - INDEX_HEADER
                || !(index.len() - INDEX_HEADER).is_multiple_of(INDEX_ENTRY)
            {
                return Err(invalid());
            }
            let id = channel(index);
            let slot = channels
                .binary_search_by_key(&id, |cursor| cursor.channel)
                .map_err(|_| invalid())?;
            let cursor = &mut channels[slot];
            if cursor.end != 0 {
                return Err(invalid());
            }
            cursor.next = offset + INDEX_HEADER;
            cursor.end = offset + index.len();
            offset += index.len();
        }
        let index_end = offset;
        if offset < bytes.len() {
            let padding = next(bytes, offset)?;
            if padding[0] != 0x80 || padding.len() != bytes.len() - offset {
                return Err(invalid());
            }
        }
        let indexed = index_end != chunk.len();
        let (min, max, _) = messages(&chunk[HEADER..], |position, msg| {
            let id = channel(msg);
            let slot = channels
                .binary_search_by_key(&id, |cursor| cursor.channel)
                .map_err(|_| invalid())?;
            if indexed {
                let cursor = &mut channels[slot];
                if cursor.end - cursor.next < INDEX_ENTRY {
                    return Err(invalid());
                }
                let entry = &bytes[cursor.next..cursor.next + INDEX_ENTRY];
                if u64_at(entry, 0) != u64_at(msg, 15) || u64_at(entry, 8) != position as u64 {
                    return Err(invalid());
                }
                cursor.next += INDEX_ENTRY;
            }
            Ok(())
        })?;
        if u64_at(chunk, 9) != min
            || u64_at(chunk, 17) != max
            || channels.iter().any(|cursor| cursor.next != cursor.end)
        {
            return Err(invalid());
        }
        Ok(Self {
            bytes,
            end: chunk.len(),
            index_end,
        })
    }
    /// Stage a Chunk Index before admission; the actual file range patches offsets.
    pub fn stage(&self, summary: &mut Vec<u8>) -> io::Result<usize> {
        let mut channels = 0;
        let mut offset = self.end;
        while offset < self.index_end {
            channels += 1;
            offset += next(self.bytes, offset)?.len();
        }
        let size = 73 + 10 * channels;
        summary.try_reserve(size).map_err(io::Error::other)?;
        let start = summary.len();
        summary.resize(start + size, 0);
        let out = &mut summary[start..];
        out[0] = 8;
        out[1..9].copy_from_slice(&((size - 9) as u64).to_le_bytes());
        out[9..25].copy_from_slice(&self.bytes[9..25]);
        out[33..41].copy_from_slice(&(self.end as u64).to_le_bytes());
        out[41..45].copy_from_slice(&((channels * 10) as u32).to_le_bytes());
        offset = self.end;
        for i in 0..channels {
            let index = next(self.bytes, offset)?;
            let field = 45 + 10 * i;
            out[field..field + 2].copy_from_slice(&index[9..11]);
            out[field + 2..field + 10].copy_from_slice(&(offset as u64).to_le_bytes());
            offset += index.len();
        }
        let pos = 45 + 10 * channels;
        out[pos..pos + 8].copy_from_slice(&((self.index_end - self.end) as u64).to_le_bytes());
        let raw = u64_at(self.bytes, 25);
        out[pos + 12..pos + 20].copy_from_slice(&raw.to_le_bytes());
        out[pos + 20..pos + 28].copy_from_slice(&raw.to_le_bytes());
        Ok(start)
    }
}

pub(super) fn accepted(summary: &mut [u8], start: usize, file_offset: u64) {
    let out = &mut summary[start..];
    out[25..33].copy_from_slice(&file_offset.to_le_bytes());
    let channels = u32_at(out, 41) as usize / 10;
    for i in 0..channels {
        let pos = 47 + 10 * i;
        let relative = u64_at(out, pos);
        out[pos..pos + 8].copy_from_slice(&(file_offset + relative).to_le_bytes());
    }
}

pub(super) fn summary_offset(out: &mut Vec<u8>, opcode: u8, start: u64, len: u64) {
    let mut content = [0; 17];
    content[0] = opcode;
    content[1..9].copy_from_slice(&start.to_le_bytes());
    content[9..17].copy_from_slice(&len.to_le_bytes());
    record(out, 14, &content);
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcap::{read::LinearReader, records::Record};

    #[test]
    fn every_alignment_remainder_keeps_the_same_extent() {
        for payload in 0..ALIGNMENT * 2 {
            // Arrange: two channels, including reversed timestamps.
            let mut records = Vec::new();
            for (id, time, len) in [(1_u16, 90_u64, payload), (2, 7, 0)] {
                let mut content = Vec::new();
                content.extend_from_slice(&id.to_le_bytes());
                content.extend_from_slice(&0_u32.to_le_bytes());
                content.extend_from_slice(&time.to_le_bytes());
                content.extend_from_slice(&time.to_le_bytes());
                content.resize(22 + len, 0xac);
                record(&mut records, 5, &content);
            }
            let end = HEADER + records.len();
            let physical = checked_padded_len(end).unwrap();
            let slack = physical - end;
            let expected = slack >= 62 && (slack == 62 || slack >= 71);
            let mut bytes = vec![0; physical];
            bytes[HEADER..end].copy_from_slice(&records);

            // Act
            assert_eq!(frame(&mut bytes, end).unwrap(), physical);
            let view = View::parse(&bytes, &mut cursors(&[1, 2]).unwrap()).unwrap();
            let decoded = LinearReader::sans_magic(&bytes)
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

            // Assert: independent decoder sees complete indexes, or none.
            assert_eq!(&bytes[HEADER..end], records);
            assert_eq!(view.end, end);
            let indexes: Vec<_> = decoded
                .iter()
                .filter_map(|r| match r {
                    Record::MessageIndex(i) => Some(i),
                    _ => None,
                })
                .collect();
            assert_eq!(
                indexes.len(),
                if expected { 2 } else { 0 },
                "payload {payload}, slack {slack}"
            );
            if expected {
                assert_eq!(indexes[0].records[0].offset, 0);
                assert_eq!(indexes[1].records[0].offset, (31 + payload) as u64);
                assert_eq!(indexes[0].records[0].log_time, 90);
                assert_eq!(indexes[1].records[0].log_time, 7);
            }
            let chunk = mcap::read::parse_record(6, &bytes[9..end]).unwrap();
            if let Record::Chunk { header, .. } = chunk {
                assert_eq!(
                    (header.message_start_time, header.message_end_time),
                    (7, 90)
                );
            } else {
                panic!("missing chunk");
            }
        }
    }
}
