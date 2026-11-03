#[path = "support/allocations.rs"]
mod allocations;

use prost::Message as _;
use storage_persistence::mcap::{Channel, McapWriter, Message, Schema};
#[derive(Clone, PartialEq, prost::Message)]
struct Payload {
    #[prost(bytes = "vec", tag = "1")]
    data: Vec<u8>,
}
#[test]
fn raw_records_are_written_unchanged_and_malformed_ones_are_refused() {
    // Arrange
    let file =
        |name: &str| std::env::temp_dir().join(format!("doramcap-{name}-{}", std::process::id()));
    let framed = |op: u8, content: &[u8]| {
        let mut out = vec![op];
        out.extend_from_slice(&(content.len() as u64).to_le_bytes());
        out.extend_from_slice(content);
        out
    };
    let text = |value: &str| {
        let mut out = (value.len() as u32).to_le_bytes().to_vec();
        out.extend_from_slice(value.as_bytes());
        out
    };
    let schema = [
        &1_u16.to_le_bytes()[..],
        &text("S"),
        &text("raw"),
        &text("schema"),
    ]
    .concat();
    let channel = [
        &7_u16.to_le_bytes()[..],
        &0_u16.to_le_bytes(),
        &text("no-schema"),
        &text("raw"),
        &0_u32.to_le_bytes(),
    ]
    .concat();
    let records = [framed(3, &schema), framed(4, &channel)].concat();
    let config = || storage_persistence::mcap::McapConfig::default();
    // Act
    let path = file("records");
    let mut writer = McapWriter::with_records(&path, &records, config()).unwrap();
    let message = Message {
        channel: 7,
        sequence: 0,
        log_time: 0,
        publish_time: 0,
    };
    writer.reserve(message, 1).unwrap().commit().unwrap();
    writer.finish().unwrap();
    // Assert
    let bytes = std::fs::read(&path).unwrap();
    let messages = mcap::MessageStream::new(&bytes)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(messages[0].channel.topic, "no-schema");
    assert!(messages[0].channel.schema.is_none());
    std::fs::remove_file(path).unwrap();
    // Act / Assert: no channels, bad framing, other records and duplicate IDs.
    let duplicate = [framed(4, &channel), framed(4, &channel)].concat();
    let footer = framed(2, &[0; 20]);
    let truncated = &records[..records.len() - 1];
    for bad in [&[][..], truncated, &footer, &duplicate] {
        let path = file("malformed");
        assert!(McapWriter::with_records(&path, bad, config()).is_err());
        assert!(!path.exists());
    }
}
#[test]
fn protobuf_roundtrip_boundaries_and_abandonment() {
    for empty in [true, false] {
        let path =
            std::env::temp_dir().join(format!("doramcap-format-{}-{empty}", std::process::id()));
        let mut writer = McapWriter::create(
            &path,
            Schema {
                name: "Payload",
                encoding: "protobuf",
                data: b"schema",
            },
            &[Channel {
                id: 1,
                topic: "test",
                encoding: "protobuf",
            }],
            storage_persistence::mcap::McapConfig {
                buffers: 2,
                buffer_size: 8192,
                io: Default::default(),
                ..Default::default()
            },
        )
        .unwrap();
        let meta = Message {
            channel: 1,
            sequence: 7,
            log_time: 100,
            publish_time: 101,
        };
        drop(writer.reserve(meta, 4066).unwrap());
        assert!(writer.reserve(Message { channel: 2, ..meta }, 0).is_err());
        {
            let mut abandoned = writer.reserve(meta, 12).unwrap();
            abandoned.payload_mut().fill(0xff);
        }
        let sizes = [0, 1, 127, 128, 1024, 4062];
        if !empty {
            for size in sizes {
                let p = Payload {
                    data: vec![42; size],
                };
                writer.flush_written().unwrap();
                allocations::start();
                let mut reservation = writer.reserve(meta, p.encoded_len()).unwrap();
                let original = reservation.payload().as_ptr();
                p.encode(&mut reservation.payload_mut()).unwrap();
                assert_eq!(reservation.payload().as_ptr(), original);
                reservation.commit().unwrap();
                let allocations = allocations::finish();
                assert_eq!(allocations, 0);
            }
        }
        writer.finish().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[bytes.len() - 8..], b"\x89MCAP0\r\n");
        let messages = mcap::MessageStream::new(&bytes)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(messages.len(), if empty { 0 } else { sizes.len() });
        for (message, size) in messages.iter().zip(sizes) {
            assert_eq!(
                message.channel.schema.as_ref().unwrap().data.as_ref(),
                b"schema"
            );
            assert_eq!(
                (message.sequence, message.log_time, message.publish_time),
                (7, 100, 101)
            );
            assert_eq!(
                Payload::decode(message.data.as_ref()).unwrap().data,
                vec![42; size]
            );
        }
        std::fs::remove_file(path).unwrap();
    }
}

fn direct_writer(name: &str, registrations: u32) -> (std::path::PathBuf, McapWriter) {
    let path = std::env::temp_dir().join(format!("doramcap-{name}-{}", std::process::id()));
    let writer = McapWriter::create(
        &path,
        Schema {
            name: "S",
            encoding: "raw",
            data: b"s",
        },
        &[Channel {
            id: 1,
            topic: "t",
            encoding: "raw",
        }],
        storage_persistence::mcap::McapConfig {
            buffers: 4,
            buffer_size: 8192,
            io: storage_persistence::writer::WriterConfig {
                registration_capacity: registrations,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    (path, writer)
}
const MESSAGE: Message = Message {
    channel: 1,
    sequence: 0,
    log_time: 0,
    publish_time: 0,
};

#[test]
fn write_keeps_call_order_across_large_payloads() {
    // Arrange
    let (path, mut writer) = direct_writer("write-order", 1024);
    // Act
    for (byte, size) in [(1, 8), (2, 64 * 1024), (3, 8)] {
        writer.write(MESSAGE, &vec![byte; size]).unwrap();
    }
    writer.finish().unwrap();
    // Assert
    let bytes = std::fs::read(&path).unwrap();
    let order: Vec<_> = mcap::MessageStream::new(&bytes)
        .unwrap()
        .map(|message| message.unwrap().data[0])
        .collect();
    assert_eq!(order, [1, 2, 3]);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn reserve_keeps_commit_order_across_large_payloads() {
    // Arrange
    let (path, mut writer) = direct_writer("reserve-order", 1024);
    // Act
    for (byte, size) in [(1, 8), (2, 64 * 1024), (3, 8)] {
        let mut reservation = writer.reserve(MESSAGE, size).unwrap();
        reservation.payload_mut().fill(byte);
        reservation.commit().unwrap();
    }
    writer.finish().unwrap();
    // Assert
    let bytes = std::fs::read(&path).unwrap();
    let order: Vec<_> = mcap::MessageStream::new(&bytes)
        .unwrap()
        .map(|message| message.unwrap().data[0])
        .collect();
    assert_eq!(order, [1, 2, 3]);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn write_fails_when_registrations_stay_full() {
    // Arrange: a small table, which growing payloads fill with retained allocations.
    let registrations = 8;
    let (path, mut writer) = direct_writer("write-full", registrations);
    // Act
    let error = (1..=registrations as usize + 1)
        .try_for_each(|size| writer.write(MESSAGE, &vec![7; size * 64 * 1024]))
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn existing_paths_are_refused_and_failed_rotation_keeps_recording() {
    // Arrange
    let (path, mut writer) = direct_writer("rotate-existing", 1024);
    // Act / Assert: the open path cannot be recreated or rotated into.
    let schema = Schema {
        name: "S",
        encoding: "raw",
        data: b"s",
    };
    let channels = [Channel {
        id: 1,
        topic: "t",
        encoding: "raw",
    }];
    let created = McapWriter::create(&path, schema, &channels, Default::default());
    let existing = std::io::ErrorKind::AlreadyExists;
    assert_eq!(created.err().unwrap().kind(), existing);
    assert_eq!(writer.rotate(&path).unwrap_err().kind(), existing);
    // Act
    writer.write(MESSAGE, &[9]).unwrap();
    writer.finish().unwrap();
    // Assert
    let bytes = std::fs::read(&path).unwrap();
    let messages = mcap::MessageStream::new(&bytes)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].data.as_ref(), &[9]);
    std::fs::remove_file(path).unwrap();
}
