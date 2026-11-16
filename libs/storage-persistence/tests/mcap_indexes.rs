mod support;
use storage_persistence::{
    mcap::{Channel, McapConfig, McapWriter, Message, PreparedBlock, Schema},
    slabs::SlabPool,
};

#[test]
fn caller_buffers_are_validated_and_submitted_without_copying() {
    for corruption in [None, Some(0), Some(1), Some(9)] {
        // Arrange
        let path = std::env::temp_dir().join(format!(
            "mcap-caller-buffer-{}-{corruption:?}",
            std::process::id()
        ));
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
                ..Default::default()
            },
        )
        .unwrap();
        let pool = SlabPool::new(1, 4096, 4096).unwrap();
        let mut slab = pool.acquire().unwrap();
        let len = support::stamp(
            slab.bytes_mut(),
            Message {
                channel: 1,
                sequence: 7,
                log_time: 10,
                publish_time: 11,
            },
            &[0x77],
        );
        if let Some(offset) = corruption {
            slab.bytes_mut()[offset] ^= 0x40;
        }
        let block = PreparedBlock::from_slab(slab.seal(len).unwrap());
        let reader = block.slab().clone();
        let address = reader.as_slice().as_ptr();
        let original = reader.as_slice().to_vec();
        let before = writer.file_bytes();
        // Act
        let result = writer.append_prepared_records(block);
        // Assert
        if corruption.is_some() {
            let rejected = result.unwrap_err();
            assert_eq!(rejected.error.kind(), std::io::ErrorKind::InvalidInput);
            assert_eq!(rejected.value.slab().as_slice().as_ptr(), address);
            assert_eq!(writer.file_bytes(), before);
        } else {
            let range = result.unwrap();
            writer.flush_written().unwrap();
            let bytes = std::fs::read(&path).unwrap();
            assert_eq!(
                &bytes[range.offset as usize..(range.offset + range.len) as usize],
                original
            );
        }
        assert_eq!(reader.as_slice().as_ptr(), address);
        assert_eq!(reader.as_slice(), original);
        drop(reader);
        writer.finish().unwrap();
        assert_eq!(pool.available(), 1);
        let bytes = std::fs::read(&path).unwrap();
        let messages = mcap::MessageStream::new(&bytes)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(messages.len(), usize::from(corruption.is_none()));
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn final_summaries_stream_across_writes_and_rotations() {
    use mcap::{
        read::{LinearReader, Summary},
        records::Record,
    };
    // Arrange: declaration summaries exceed one managed write.
    let path =
        std::env::temp_dir().join(format!("mcap-summary-declarations-{}", std::process::id()));
    let rotated = path.with_extension("rotated");
    let schema_data = vec![0x42; 8192 * 3];
    let mut writer = McapWriter::create(
        &path,
        Schema {
            name: "opaque",
            encoding: "",
            data: &schema_data,
        },
        &[Channel {
            id: 1,
            topic: "one",
            encoding: "",
        }],
        McapConfig {
            buffers: 2,
            buffer_size: 8192,
            ..Default::default()
        },
    )
    .unwrap();
    // Act
    for sequence in 0..2 {
        if sequence == 1 {
            writer.rotate(&rotated).unwrap();
        }
        writer
            .write(
                Message {
                    channel: 1,
                    sequence,
                    log_time: 10,
                    publish_time: 11,
                },
                &[0x77],
            )
            .unwrap();
    }
    writer.finish().unwrap();
    // Assert with official sequential and summary readers.
    for file in [&path, &rotated] {
        let bytes = std::fs::read(file).unwrap();
        let summary = Summary::read(&bytes).unwrap().unwrap();
        assert_eq!(summary.schemas[&1].data.as_ref(), schema_data);
        assert_eq!(summary.channels.len(), 1);
        assert!(summary.chunk_indexes.is_empty());
        assert_eq!(
            mcap::MessageStream::new(&bytes)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(&bytes[bytes.len() - 8..], b"\x89MCAP0\r\n");
        for record in LinearReader::new(&bytes).unwrap() {
            if let Record::SummaryOffset(group) = record.unwrap() {
                let start = group.group_start as usize;
                let end = start + group.group_length as usize;
                let records = LinearReader::sans_magic(&bytes[start..end])
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                assert!(!records.is_empty());
                assert!(
                    records
                        .iter()
                        .all(|record| record.opcode() == group.group_opcode)
                );
            }
        }
        std::fs::remove_file(file).unwrap();
    }
}
