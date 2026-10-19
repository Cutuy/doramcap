use std::{fs::File, os::unix::fs::OpenOptionsExt};
use storage_persistence::{
    managed_buffers::BufferPool,
    mcap::{Channel, McapWriter, Message, Schema},
    writer::Writer,
};
#[test]
fn aligned_mcap_padding_and_exact_tail() {
    for alignment in [512, 4096] {
        let path = std::env::temp_dir().join(format!(
            "doramcap-direct-{}-{alignment}",
            std::process::id()
        ));
        let open = || {
            File::options()
                .create_new(true)
                .write(true)
                .custom_flags(libc::O_DIRECT)
                .open(&path)
                .unwrap()
        };
        let mut writer = McapWriter::direct(
            open(),
            2,
            alignment * 2,
            Schema {
                name: "opaque",
                encoding: "",
                data: b"",
            },
            &[Channel {
                id: 1,
                topic: "test",
                encoding: "",
            }],
            alignment,
        )
        .unwrap();
        let meta = Message {
            channel: 1,
            sequence: 0,
            log_time: 0,
            publish_time: 0,
        };
        let sizes = [
            0,
            1,
            alignment - 40,
            alignment - 39,
            alignment - 32,
            alignment - 31,
        ];
        for size in sizes {
            let mut r = writer.reserve(meta, size).unwrap();
            r.payload_mut().fill(42);
            r.commit().unwrap();
            writer.flush_written().unwrap();
        }
        writer.finish().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[bytes.len() - 8..], b"\x89MCAP0\r\n");
        assert_eq!(bytes.len() % alignment, 50);
        let msgs = mcap::MessageStream::new(&bytes)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(msgs.len(), sizes.len());
        for (msg, size) in msgs.iter().zip(sizes) {
            assert_eq!(msg.data.as_ref(), vec![42; size]);
        }
        std::fs::remove_file(&path).unwrap();
        let pool = BufferPool::new(2, alignment * 2, 4096).unwrap();
        let mut raw = Writer::direct(open(), pool.clone(), alignment).unwrap();
        assert!(raw.append(pool.reserve(1).unwrap()).is_err());
        let foreign = BufferPool::new(1, alignment, 4096).unwrap();
        assert!(raw.append(foreign.reserve(alignment).unwrap()).is_err());
        raw.append(pool.reserve(alignment).unwrap()).unwrap();
        raw.close_at(1).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 1);
        assert_eq!(pool.available(), 2);
        std::fs::remove_file(path).unwrap();
    }
}
