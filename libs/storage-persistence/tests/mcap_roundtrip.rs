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
    let file = |name: &str| {
        let path = std::env::temp_dir().join(format!("doramcap-{name}-{}", std::process::id()));
        let opened = std::fs::File::options()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        (path, opened)
    };
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
    let (path, opened) = file("records");
    let mut writer = McapWriter::with_records(opened, &records, config()).unwrap();
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
        let (path, opened) = file("malformed");
        assert!(McapWriter::with_records(opened, bad, config()).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
#[test]
fn protobuf_roundtrip_boundaries_and_abandonment() {
    for empty in [true, false] {
        let path =
            std::env::temp_dir().join(format!("doramcap-format-{}-{empty}", std::process::id()));
        let file = std::fs::File::options()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        let mut writer = McapWriter::new(
            file,
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
                buffer_size: 4096,
                alignment: 1,
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
