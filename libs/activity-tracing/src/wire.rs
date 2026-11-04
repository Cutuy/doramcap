//! Versioned, self-contained Protobuf batches. Dictionary IDs are zero-based.
use crate::capture::{Block, Core, FIELDS, Field, Losses, Record};
use std::ops::Deref;
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/doramcap.activity.v1.rs"));
}
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/activity.bin"));
pub const SCHEMA: &str = "doramcap.activity.v1.Batch";
pub const VERSION: u32 = 1;
/// Receives encoded bytes; a [`Counter`] measures what a [`Slice`] then writes.
trait Out {
    fn put(&mut self, bytes: &[u8]);
}
struct Counter(usize);
impl Out for Counter {
    fn put(&mut self, bytes: &[u8]) {
        self.0 += bytes.len();
    }
}
struct Slice<'a>(&'a mut [u8]);
impl Out for Slice<'_> {
    fn put(&mut self, bytes: &[u8]) {
        let (head, tail) = std::mem::take(&mut self.0).split_at_mut(bytes.len());
        head.copy_from_slice(bytes);
        self.0 = tail;
    }
}
fn varint(mut n: u64, out: &mut impl Out) {
    let mut buffer = [0; 10];
    let mut len = 0;
    while n >= 0x80 {
        buffer[len] = n as u8 | 0x80;
        n >>= 7;
        len += 1;
    }
    buffer[len] = n as u8;
    out.put(&buffer[..=len]);
}
fn number(tag: u32, n: u64, out: &mut impl Out) {
    varint((tag as u64) << 3, out);
    varint(n, out);
}
fn fixed(tag: u32, n: u64, out: &mut impl Out) {
    varint((tag as u64) << 3 | 1, out);
    out.put(&n.to_le_bytes());
}
fn prefix(tag: u32, len: usize, out: &mut impl Out) {
    varint((tag as u64) << 3 | 2, out);
    varint(len as u64, out);
}
fn bytes(tag: u32, b: &[u8], out: &mut impl Out) {
    prefix(tag, b.len(), out);
    out.put(b);
}
fn id(names: &[&'static str], s: &'static str) -> u64 {
    names.iter().position(|n| *n == s).unwrap() as u64
}
fn field(names: &[&'static str], f: &Field, base: usize, out: &mut impl Out) {
    number(1, id(names, f.name), out);
    number(2, f.kind as u64, out);
    fixed(3, f.bits, out);
    number(4, f.offset as u64 + base as u64, out);
    number(5, f.len as u64, out);
}
fn record(names: &[&'static str], r: &Record, base: usize, out: &mut impl Out) {
    for (tag, n) in [
        (1, r.kind as u64),
        (2, r.time),
        (3, r.span),
        (4, r.root),
        (5, r.parent),
        (6, r.linked),
        (7, r.linked_root),
        (8, r.tid as u64),
    ] {
        number(tag, n, out);
    }
    number(9, id(names, r.thread), out);
    number(10, id(names, r.name), out);
    for f in &r.fields[..r.count] {
        let mut len = Counter(0);
        field(names, f, base, &mut len);
        prefix(11, len.0, out);
        field(names, f, base, out);
    }
}
/// The size and header of one message over the first `blocks` measured blocks.
pub(crate) struct Plan {
    pub blocks: usize,
    pub len: usize,
    pub sequence: u64,
    pub unix_ns: u64,
    losses: Losses,
}
pub(crate) struct Encoder {
    names: Vec<&'static str>,
    /// Encoded length of each planned record, reused by [`Self::write`].
    lengths: Vec<usize>,
}
impl Encoder {
    pub fn new(records: usize) -> Self {
        Self {
            names: Vec::with_capacity(records * (FIELDS + 2)),
            lengths: Vec::with_capacity(records),
        }
    }
    /// Adds `s` to the dictionary; returns the encoded bytes this adds.
    fn intern(&mut self, s: &'static str) -> usize {
        if self.names.contains(&s) {
            return 0;
        }
        self.names.push(s);
        let mut len = Counter(0);
        bytes(7, s.as_bytes(), &mut len);
        len.0
    }
    fn header(core: &Core, sequence: u64, losses: &Losses, out: &mut impl Out) {
        number(1, VERSION as u64, out);
        fixed(2, core.session, out);
        number(3, sequence, out);
        number(4, core.unix_origin, out);
        number(5, losses.records, out);
        number(6, std::process::id() as u64, out);
        number(10, losses.fields, out);
        number(11, losses.spans, out);
        number(12, losses.unregistered, out);
    }
    /// Measures the longest prefix of nonempty `blocks` fitting `limit` bytes, always taking
    /// the first block. Pass the same prefix to [`Self::write`] before planning again.
    pub fn plan<B: Deref<Target = Block>>(
        &mut self,
        core: &Core,
        blocks: &[B],
        sequence: u64,
        limit: usize,
    ) -> Plan {
        let losses = core.losses.snapshot();
        let mut header = Counter(0);
        Self::header(core, sequence, &losses, &mut header);
        let unix_ns = core
            .unix_origin
            .saturating_add(blocks[0].records.first().map_or(0, |r| r.time));
        self.names.clear();
        self.lengths.clear();
        let (mut count, mut len, mut dictionary, mut strings, mut records) = (0, 0, 0, 0, 0);
        for block in blocks {
            let names = self.names.len();
            let lengths = self.lengths.len();
            let mut grown = 0;
            for r in &block.records {
                grown += self.intern(r.thread) + self.intern(r.name);
                for f in &r.fields[..r.count] {
                    grown += self.intern(f.name);
                }
            }
            let mut framed = 0;
            for r in &block.records {
                let mut body = Counter(0);
                record(&self.names, r, strings, &mut body);
                self.lengths.push(body.0);
                let mut with_prefix = Counter(body.0);
                prefix(9, body.0, &mut with_prefix);
                framed += with_prefix.0;
            }
            let next_strings = strings + block.strings.len();
            let mut arena = Counter(next_strings);
            prefix(8, next_strings, &mut arena);
            let next = header.0 + dictionary + grown + arena.0 + records + framed;
            if count > 0 && next > limit {
                self.names.truncate(names);
                self.lengths.truncate(lengths);
                break;
            }
            count += 1;
            len = next;
            dictionary += grown;
            strings = next_strings;
            records += framed;
        }
        Plan {
            blocks: count,
            len,
            sequence,
            unix_ns,
            losses,
        }
    }
    /// Serializes the blocks `plan` measured; `out` must be exactly `plan.len` bytes.
    pub fn write<B: Deref<Target = Block>>(
        &self,
        core: &Core,
        blocks: &[B],
        plan: &Plan,
        out: &mut [u8],
    ) {
        let mut out = Slice(out);
        Self::header(core, plan.sequence, &plan.losses, &mut out);
        for name in &self.names {
            bytes(7, name.as_bytes(), &mut out);
        }
        // One string arena: each block's offsets shift by the bytes before it.
        prefix(8, blocks.iter().map(|b| b.strings.len()).sum(), &mut out);
        for block in blocks {
            out.put(&block.strings);
        }
        let mut base = 0;
        let mut lengths = self.lengths.iter();
        for block in blocks {
            for r in &block.records {
                prefix(9, *lengths.next().unwrap(), &mut out);
                record(&self.names, r, base, &mut out);
            }
            base += block.strings.len();
        }
        assert!(out.0.is_empty());
    }
}
/// Decode and validate external or recorded bytes before using dictionary/arena offsets.
pub fn decode(bytes: &[u8]) -> crate::Result<proto::Batch> {
    use prost::Message;
    let b = proto::Batch::decode(bytes)?;
    if b.version != VERSION || b.session == 0 {
        return Err("unsupported activity batch".into());
    }
    for r in &b.records {
        if r.name as usize >= b.dictionary.len()
            || r.thread_name as usize >= b.dictionary.len()
            || !(0..=4).contains(&r.kind)
        {
            return Err("invalid record metadata".into());
        }
        for f in &r.fields {
            if f.name as usize >= b.dictionary.len() || !(0..=4).contains(&f.kind) {
                return Err("invalid field metadata".into());
            }
            if f.kind == 4 {
                let end = (f.offset as usize)
                    .checked_add(f.length as usize)
                    .ok_or("string range overflow")?;
                std::str::from_utf8(
                    b.strings
                        .get(f.offset as usize..end)
                        .ok_or("invalid string range")?,
                )?;
            }
        }
    }
    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    #[test]
    fn invalid_dictionary_and_arena_ranges_are_rejected() {
        let mut b = proto::Batch {
            version: VERSION,
            session: 1,
            dictionary: vec!["x".into()],
            records: vec![proto::Record::default()],
            ..Default::default()
        };
        assert!(decode(&b.encode_to_vec()).is_ok());
        b.records[0].name = 9;
        assert!(decode(&b.encode_to_vec()).is_err());
        b.records[0].name = 0;
        b.records[0].fields.push(proto::Field {
            kind: 4,
            length: 8,
            ..Default::default()
        });
        assert!(decode(&b.encode_to_vec()).is_err());
        b.strings = vec![255; 8];
        assert!(decode(&b.encode_to_vec()).is_err());
    }
}
