//! Versioned, self-contained Protobuf batches. Dictionary IDs are zero-based.
use crate::capture::{Block, Core, FIELDS};
use prost::bytes::BufMut;
use prost::encoding::{WireType, encode_key, encode_varint};
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/doramcap.activity.v1.rs"));
}
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/activity.bin"));
pub const SCHEMA: &str = "doramcap.activity.v1.Batch";
pub const VERSION: u32 = 1;
fn number(tag: u32, n: u64, out: &mut Vec<u8>) {
    encode_key(tag, WireType::Varint, out);
    encode_varint(n, out);
}
fn fixed(tag: u32, n: u64, out: &mut Vec<u8>) {
    encode_key(tag, WireType::SixtyFourBit, out);
    out.put_u64_le(n);
}
fn bytes(tag: u32, b: &[u8], out: &mut Vec<u8>) {
    encode_key(tag, WireType::LengthDelimited, out);
    encode_varint(b.len() as u64, out);
    out.extend_from_slice(b);
}
pub(crate) struct Encoder {
    names: Vec<&'static str>,
    record: Vec<u8>,
    field: Vec<u8>,
}
impl Encoder {
    pub fn new(records: usize) -> Self {
        Self {
            names: Vec::with_capacity(records * (FIELDS + 2)),
            record: Vec::with_capacity(2048),
            field: Vec::with_capacity(128),
        }
    }
    fn intern(&mut self, s: &'static str) -> u64 {
        if let Some(i) = self.names.iter().position(|n| *n == s) {
            i as u64
        } else {
            self.names.push(s);
            (self.names.len() - 1) as u64
        }
    }
    pub fn encode(&mut self, core: &Core, block: &Block, sequence: u64, out: &mut Vec<u8>) {
        out.clear();
        self.names.clear();
        for r in &block.records {
            self.intern(r.thread);
            self.intern(r.name);
            for f in &r.fields[..r.count] {
                self.intern(f.name);
            }
        }
        number(1, VERSION as u64, out);
        fixed(2, core.session, out);
        number(3, sequence, out);
        number(4, core.unix_origin, out);
        number(5, core.losses.snapshot().records, out);
        number(6, std::process::id() as u64, out);
        let losses = core.losses.snapshot();
        number(10, losses.fields, out);
        number(11, losses.spans, out);
        number(12, losses.unregistered, out);
        for name in &self.names {
            bytes(7, name.as_bytes(), out);
        }
        bytes(8, &block.strings, out);
        for r in &block.records {
            self.record.clear();
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
                number(tag, n, &mut self.record);
            }
            let thread = self.intern(r.thread);
            let name = self.intern(r.name);
            number(9, thread, &mut self.record);
            number(10, name, &mut self.record);
            for f in &r.fields[..r.count] {
                let name = self.intern(f.name);
                self.field.clear();
                number(1, name, &mut self.field);
                number(2, f.kind as u64, &mut self.field);
                fixed(3, f.bits, &mut self.field);
                number(4, f.offset as u64, &mut self.field);
                number(5, f.len as u64, &mut self.field);
                bytes(11, &self.field, &mut self.record);
            }
            bytes(9, &self.record, out);
        }
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
