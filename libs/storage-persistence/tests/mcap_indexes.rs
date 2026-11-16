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
