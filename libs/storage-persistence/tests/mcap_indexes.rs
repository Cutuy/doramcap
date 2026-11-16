use mcap::{
    read::{LinearReader, Summary},
    records::Record,
};
use storage_persistence::{
    mcap::{Channel, McapConfig, McapWriter, Message, PreparedBlock, Schema},
    slabs::SlabPool,
};

fn message(sequence: u32, channel: u16) -> Message {
    Message {
        sequence,
        channel,
        log_time: 100 - sequence as u64,
        publish_time: 200,
    }
}
fn writer(path: &std::path::Path) -> McapWriter {
    McapWriter::create(
        path,
        Schema {
            name: "opaque",
            encoding: "",
            data: b"",
        },
        &[
            Channel {
                id: 1,
                topic: "one",
                encoding: "",
            },
            Channel {
                id: 2,
                topic: "two",
                encoding: "",
            },
        ],
        McapConfig {
            buffers: 2,
            buffer_size: 8192,
            ..Default::default()
        },
    )
    .unwrap()
}

// Independently authored caller bytes: no writer builder or internal encoder.
fn caller_chunk(bytes: &mut [u8], indexed: bool) {
    bytes.fill(0);
    bytes[0] = 6;
    bytes[1..9].copy_from_slice(&72_u64.to_le_bytes());
    bytes[9..17].copy_from_slice(&97_u64.to_le_bytes());
    bytes[17..25].copy_from_slice(&97_u64.to_le_bytes());
    bytes[25..33].copy_from_slice(&32_u64.to_le_bytes());
    bytes[41..49].copy_from_slice(&32_u64.to_le_bytes());
    bytes[49] = 5;
    bytes[50..58].copy_from_slice(&23_u64.to_le_bytes());
    bytes[58..60].copy_from_slice(&2_u16.to_le_bytes());
    bytes[60..64].copy_from_slice(&3_u32.to_le_bytes());
    bytes[64..72].copy_from_slice(&97_u64.to_le_bytes());
    bytes[72..80].copy_from_slice(&200_u64.to_le_bytes());
    bytes[80] = 0x77;
    let pad = if indexed {
        bytes[81] = 7;
        bytes[82..90].copy_from_slice(&22_u64.to_le_bytes());
        bytes[90..92].copy_from_slice(&2_u16.to_le_bytes());
        bytes[92..96].copy_from_slice(&16_u32.to_le_bytes());
        bytes[96..104].copy_from_slice(&97_u64.to_le_bytes());
        112
    } else {
        81
    };
    bytes[pad] = 0x80;
    let len = (bytes.len() - pad - 9) as u64;
    bytes[pad + 1..pad + 9].copy_from_slice(&len.to_le_bytes());
}

#[test]
fn three_paths_keep_wire_offsets_payloads_and_rotated_summaries() {
    for indexed_slab in [false, true] {
        let path = std::env::temp_dir().join(format!(
            "mcap-indexes-{}-{indexed_slab}",
            std::process::id()
        ));
        let rotated = path.with_extension("rotated");
        let mut writer = writer(&path);
        let pool = SlabPool::new(1, 4096, 4096).unwrap();
        for rotation in 0..2 {
            if rotation == 1 {
                writer.rotate(&rotated).unwrap();
            }
            // Arrange/Act: one two-channel combining chunk.
            writer.write(message(0, 1), &[1, 2]).unwrap();
            writer.write(message(1, 2), &[3]).unwrap();
            writer.flush_written().unwrap();
            // Arbitrary-length large payload: 49 + 31 + 8116 == 8196, so indexes fit.
            writer.write(message(2, 1), &vec![0x55; 8116]).unwrap();
            // Independently framed caller-owned slab, with its supplied index choice.
            let mut slab = pool.acquire().unwrap();
            caller_chunk(slab.bytes_mut(), indexed_slab);
            let original = slab.bytes_mut().to_vec();
            let block = PreparedBlock::from_slab(slab.seal(4096).unwrap());
            let reader = block.slab().clone();
            let pointer = reader.as_slice().as_ptr();
            let range = writer.append_prepared_records(block).unwrap();
            assert_eq!(reader.as_slice().as_ptr(), pointer);
            assert_eq!(reader.as_slice(), original);
            writer.flush_written().unwrap();
            let current = if rotation == 0 { &path } else { &rotated };
            let on_disk = std::fs::read(current).unwrap();
            assert_eq!(
                &on_disk[range.offset as usize..(range.offset + range.len) as usize],
                original
            );
            drop(reader);
            // Mandatory framing fills this extent exactly: optional index is omitted.
            writer.write(message(4, 2), &vec![0x66; 8112]).unwrap();
            writer.flush_written().unwrap();
        }
        writer.finish().unwrap();
        for file in [&path, &rotated] {
            // Assert with the official Rust sequential and summary readers.
            let bytes = std::fs::read(file).unwrap();
            let messages = mcap::MessageStream::new(&bytes)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(
                messages.iter().map(|m| m.sequence).collect::<Vec<_>>(),
                [0, 1, 2, 3, 4]
            );
            assert_eq!(messages[2].data.as_ref(), vec![0x55; 8116]);
            assert_eq!(messages[3].data.as_ref(), [0x77]);
            let summary = Summary::read(&bytes).unwrap().unwrap();
            assert_eq!(summary.channels.len(), 2);
            assert_eq!(summary.chunk_indexes.len(), 4);
            for (i, index) in summary.chunk_indexes.iter().enumerate() {
                let start = index.chunk_start_offset as usize;
                assert_eq!(start % 4096, 0);
                assert_eq!(bytes[start], 6);
                let chunk_end = start + index.chunk_length as usize;
                assert_eq!(
                    index.message_index_offsets.is_empty(),
                    i == 3 || (i == 2 && !indexed_slab)
                );
                if !index.message_index_offsets.is_empty() {
                    assert_eq!(
                        *index.message_index_offsets.values().min().unwrap(),
                        chunk_end as u64
                    );
                    let indexes = summary.read_message_indexes(&bytes, index).unwrap();
                    assert!(!indexes.is_empty());
                } else {
                    assert_eq!(index.message_index_length, 0);
                }
            }
            let records = LinearReader::new(&bytes)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let footer = records
                .iter()
                .find_map(|r| {
                    if let Record::Footer(f) = r {
                        Some(f)
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(bytes[footer.summary_start as usize], 3);
            assert_eq!(bytes[footer.summary_offset_start as usize], 14);
            assert_eq!(&bytes[bytes.len() - 8..], b"\x89MCAP0\r\n");
            std::fs::remove_file(file).unwrap();
        }
    }
}

#[test]
fn malformed_caller_indexes_return_the_original_slab_and_consume_no_range() {
    for corruption in [1, 25, 41, 58, 64, 90, 92, 96, 104, 112] {
        // Arrange
        let path = std::env::temp_dir().join(format!(
            "mcap-bad-index-{}-{corruption}",
            std::process::id()
        ));
        let mut writer = writer(&path);
        let pool = SlabPool::new(1, 4096, 4096).unwrap();
        let mut slab = pool.acquire().unwrap();
        caller_chunk(slab.bytes_mut(), true);
        slab.bytes_mut()[corruption] ^= 0x10;
        let block = PreparedBlock::from_slab(slab.seal(4096).unwrap());
        let original = block.slab().as_slice().to_vec();
        let pointer = block.slab().as_slice().as_ptr();
        let before = writer.file_bytes();
        // Act
        let rejected = writer.append_prepared_records(block).unwrap_err();
        // Assert
        assert_eq!(rejected.error.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(rejected.value.slab().as_slice().as_ptr(), pointer);
        assert_eq!(rejected.value.slab().as_slice(), original);
        assert_eq!(writer.file_bytes(), before);
        drop(rejected);
        writer.finish().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            Summary::read(&bytes)
                .unwrap()
                .unwrap()
                .chunk_indexes
                .is_empty()
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn summary_spans_multiple_writes_with_one_pending_slot() {
    // Arrange
    let path = std::env::temp_dir().join(format!("mcap-summary-pages-{}", std::process::id()));
    let mut writer = McapWriter::create(
        &path,
        Schema {
            name: "opaque",
            encoding: "",
            data: b"",
        },
        &[Channel {
            id: 1,
            topic: "one",
            encoding: "",
        }],
        McapConfig {
            buffers: 2,
            buffer_size: 8192,
            io: storage_persistence::writer::WriterConfig {
                pending_capacity: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    // Act: alternating fitting and omitted indexes in the combining path.
    for sequence in 0..240 {
        writer
            .write(
                message(sequence % 100, 1),
                &vec![0x99; if sequence % 2 == 0 { 1 } else { 3986 }],
            )
            .unwrap();
        writer.flush_written().unwrap();
    }
    writer.finish().unwrap();
    // Assert
    let bytes = std::fs::read(&path).unwrap();
    let summary = Summary::read(&bytes).unwrap().unwrap();
    assert_eq!(summary.chunk_indexes.len(), 240);
    assert_eq!(
        summary
            .chunk_indexes
            .iter()
            .filter(|i| i.message_index_offsets.is_empty())
            .count(),
        120
    );
    assert_eq!(mcap::MessageStream::new(&bytes).unwrap().count(), 240);
    let records = LinearReader::new(&bytes)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for record in records {
        if let Record::SummaryOffset(group) = record {
            let start = group.group_start as usize;
            let end = start + group.group_length as usize;
            let grouped = LinearReader::sans_magic(&bytes[start..end])
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert!(!grouped.is_empty());
            assert!(
                grouped
                    .iter()
                    .all(|record| record.opcode() == group.group_opcode)
            );
        }
    }
    std::fs::remove_file(path).unwrap();
}
