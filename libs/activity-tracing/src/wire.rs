//! Versioned, self-contained Protobuf batches. Dictionary IDs are zero-based.
use crate::capture::{Block, Core};
use prost::Message;
use std::ops::Deref;
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/doramcap.activity.v1.rs"));
}
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/activity.bin"));
pub const SCHEMA: &str = "doramcap.activity.v1.Batch";
pub const VERSION: u32 = 1;

/// The size and header of the generated batch ready to encode.
pub(crate) struct Plan {
    pub blocks: usize,
    pub len: usize,
    pub sequence: u64,
    pub unix_ns: u64,
}
pub(crate) struct Encoder {
    batch: proto::Batch,
}
impl Encoder {
    pub fn new(records: usize) -> Self {
        Self {
            batch: proto::Batch {
                records: Vec::with_capacity(records),
                ..Default::default()
            },
        }
    }
    fn intern(&mut self, name: &str) -> u32 {
        if let Some(id) = self.batch.dictionary.iter().position(|n| n == name) {
            return id as u32;
        }
        let id = self.batch.dictionary.len() as u32;
        self.batch.dictionary.push(name.into());
        id
    }
    fn append(&mut self, block: &Block) {
        let base = self.batch.strings.len() as u32;
        self.batch.strings.extend_from_slice(&block.strings);
        for r in &block.records {
            let record = proto::Record {
                kind: r.kind as i32,
                monotonic_ns: r.time,
                span_id: r.span,
                trace_root: r.root,
                parent_id: r.parent,
                linked_id: r.linked,
                linked_root: r.linked_root,
                tid: r.tid,
                thread_name: self.intern(r.thread),
                name: self.intern(r.name),
                fields: r.fields[..r.count]
                    .iter()
                    .map(|f| proto::Field {
                        name: self.intern(f.name),
                        kind: f.kind as i32,
                        bits: f.bits,
                        offset: f.offset + base,
                        length: f.len,
                    })
                    .collect(),
            };
            self.batch.records.push(record);
        }
    }
    /// Builds a batch from the longest prefix fitting `limit`, always taking the first block.
    pub fn plan<B: Deref<Target = Block>>(
        &mut self,
        core: &Core,
        blocks: &[B],
        sequence: u64,
        limit: usize,
    ) -> Plan {
        let losses = core.losses.snapshot();
        self.batch.version = VERSION;
        self.batch.session = core.session;
        self.batch.sequence = sequence;
        self.batch.unix_origin_ns = core.unix_origin;
        self.batch.pid = std::process::id();
        self.batch.dropped_records = losses.records;
        self.batch.dropped_fields = losses.fields;
        self.batch.rejected_spans = losses.spans;
        self.batch.unregistered_records = losses.unregistered;
        self.batch.dictionary.clear();
        self.batch.strings.clear();
        self.batch.records.clear();
        let mut plan = Plan {
            blocks: 0,
            len: 0,
            sequence,
            unix_ns: core.unix_origin,
        };
        for block in blocks {
            let (names, strings, records) = (
                self.batch.dictionary.len(),
                self.batch.strings.len(),
                self.batch.records.len(),
            );
            self.append(block);
            let len = self.batch.encoded_len();
            if plan.blocks > 0 && len > limit {
                self.batch.dictionary.truncate(names);
                self.batch.strings.truncate(strings);
                self.batch.records.truncate(records);
                break;
            }
            plan.blocks += 1;
            plan.len = len;
        }
        plan.unix_ns = core.unix_origin.saturating_add(
            self.batch
                .records
                .iter()
                .map(|r| r.monotonic_ns)
                .min()
                .unwrap_or(0),
        );
        plan
    }
    /// Encodes the latest planned batch directly into its reserved output slice.
    pub fn write(&self, mut out: &mut [u8]) {
        self.batch
            .encode(&mut out)
            .expect("batch fits its measured reservation");
        assert!(out.is_empty());
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
    fn batches_preserve_records_and_strings_across_size_limits() {
        // Arrange
        use crate::capture::{Config, Field, Record};
        let core = Core::new(Config::default()).unwrap();
        let blocks: Vec<_> = ["first", "second"]
            .into_iter()
            .map(|value| {
                let mut record = Record {
                    kind: 1,
                    thread: "worker",
                    name: value,
                    count: 1,
                    ..Default::default()
                };
                record.fields[0] = Field {
                    name: "text",
                    kind: 4,
                    len: value.len() as u32,
                    ..Default::default()
                };
                Box::new(Block {
                    records: vec![record],
                    strings: value.as_bytes().to_vec(),
                })
            })
            .collect();
        let mut encoder = Encoder::new(1);
        let read = |encoder: &Encoder, plan: &Plan| {
            let mut bytes = vec![0; plan.len];
            encoder.write(&mut bytes);
            decode(&bytes).unwrap()
        };

        // Act: even an oversized first block must make progress.
        let first = encoder.plan(&core, &blocks, 1, 0);
        let batch = read(&encoder, &first);
        // Assert
        assert_eq!(first.blocks, 1);
        assert_eq!(batch.records.len(), 1);
        assert_eq!(batch.strings, b"first");

        // Act: rejecting the second block leaves a self-contained first batch.
        let limited = encoder.plan(&core, &blocks, 1, first.len);
        let batch = read(&encoder, &limited);
        // Assert
        assert_eq!(limited.blocks, 1);
        assert_eq!(batch.dictionary[batch.records[0].name as usize], "first");
        assert!(!batch.dictionary.iter().any(|name| name == "second"));
        assert_eq!(batch.strings, b"first");

        // Act: remaining blocks can be encoded independently.
        let remaining = encoder.plan(&core, &blocks[1..], 2, usize::MAX);
        let batch = read(&encoder, &remaining);
        // Assert
        assert_eq!(batch.sequence, 2);
        assert_eq!(batch.strings, b"second");
        assert_eq!(batch.records[0].fields[0].offset, 0);

        // Act: a larger reservation combines both blocks into one protobuf batch.
        let merged = encoder.plan(&core, &blocks, 3, usize::MAX);
        let batch = read(&encoder, &merged);
        // Assert
        assert_eq!(merged.blocks, 2);
        assert_eq!(batch.sequence, 3);
        let values: Vec<_> = batch
            .records
            .iter()
            .map(|r| {
                assert_eq!(batch.dictionary[r.thread_name as usize], "worker");
                let field = &r.fields[0];
                assert_eq!(batch.dictionary[field.name as usize], "text");
                &batch.strings[field.offset as usize..(field.offset + field.length) as usize]
            })
            .collect();
        assert_eq!(values, [b"first".as_slice(), b"second".as_slice()]);
    }
    #[test]
    fn timestamp_is_the_earliest_record_in_the_accepted_blocks() {
        // Arrange
        use crate::capture::{Config, Record};
        let core = Core::new(Config::default()).unwrap();
        let blocks: Vec<_> = [[80, 60], [40, 20], [10, 5]]
            .into_iter()
            .map(|times| {
                Box::new(Block {
                    records: times
                        .into_iter()
                        .map(|time| Record {
                            time,
                            ..Default::default()
                        })
                        .collect(),
                    strings: Vec::new(),
                })
            })
            .collect();
        let mut encoder = Encoder::new(2);

        // Act: an earlier record can follow the first record within a block.
        let first = encoder.plan(&core, &blocks[..1], 1, usize::MAX);
        // Assert
        assert_eq!(first.unix_ns, core.unix_origin + 60);

        // Act: published blocks can also arrive out of timestamp order.
        let merged = encoder.plan(&core, &blocks[..2], 1, usize::MAX);
        // Assert
        assert_eq!(merged.unix_ns, core.unix_origin + 20);

        // Act: records from a block rejected by the size limit must be excluded.
        let limited = encoder.plan(&core, &blocks, 1, merged.len);
        // Assert
        assert_eq!(limited.blocks, 2);
        assert_eq!(limited.unix_ns, core.unix_origin + 20);
    }
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
