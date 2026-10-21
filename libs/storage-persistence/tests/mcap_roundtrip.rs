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
