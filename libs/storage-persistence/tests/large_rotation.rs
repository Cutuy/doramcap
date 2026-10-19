use bytes::BufMut;
use std::{fs::File, os::unix::fs::OpenOptionsExt};
use storage_persistence::mcap::{Channel, McapWriter, Message, Schema};
fn create(path: &std::path::Path, alignment: usize) -> File {
    File::options()
        .write(true)
        .create_new(true)
        .custom_flags(if alignment > 1 { libc::O_DIRECT } else { 0 })
        .open(path)
        .unwrap()
}
fn writer(file: File, alignment: usize) -> McapWriter {
    let schema = Schema {
        name: "bytes",
        encoding: "",
        data: b"descriptor",
    };
    let channels = [Channel {
        id: 1,
        topic: "test",
        encoding: "",
    }];
    if alignment > 1 {
        McapWriter::direct(file, 3, 8192, schema, &channels, alignment).unwrap()
    } else {
        McapWriter::new(file, 3, 8192, schema, &channels).unwrap()
    }
}
fn meta(sequence: u32) -> Message {
    Message {
        channel: 1,
        sequence,
        log_time: sequence as u64,
        publish_time: sequence as u64,
    }
}
#[test]
fn segmented_records_and_rotation() {
    for alignment in [1, 512, 4096] {
        let path = |i| {
            std::env::temp_dir().join(format!(
                "doramcap-large-{}-{alignment}-{i}",
                std::process::id()
            ))
        };
        let mut writer = writer(create(&path(0), alignment), alignment);
        assert!(
            writer
                .reserve_large(meta(0), writer.max_large_payload() + 1)
                .is_err()
        );
        {
            let mut r = writer.reserve_large(meta(0), 9000).unwrap();
            r.put_bytes(0xff, 10);
            assert!(r.commit().is_err());
        }
        for i in 0..16 {
            let size = [8190, 8192, 16383, 20000][i % 4];
            let mut r = writer.reserve_large(meta(i as u32), size).unwrap();
            r.put_bytes(i as u8, size);
            r.commit().unwrap();
            let mut small = writer.reserve(meta(i as u32), 1).unwrap();
            small.payload_mut()[0] = 42;
            small.commit().unwrap();
            if i < 15 {
                writer.rotate(create(&path(i + 1), alignment)).unwrap();
            }
        }
        writer.finish().unwrap();
        for i in 0..16 {
            let data = std::fs::read(path(i)).unwrap();
            let msgs = mcap::MessageStream::new(&data)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(msgs.len(), 2);
            assert_eq!(msgs[0].data.len(), [8190, 8192, 16383, 20000][i % 4]);
            assert!(msgs[0].data.iter().all(|b| *b == i as u8));
            assert_eq!(msgs[0].sequence, i as u32);
            assert_eq!(msgs[1].data.as_ref(), &[42]);
            assert_eq!(
                msgs[0].channel.schema.as_ref().unwrap().data.as_ref(),
                b"descriptor"
            );
            std::fs::remove_file(path(i)).unwrap();
        }
    }
}
#[test]
fn abrupt_exit_preserves_complete_record_prefix() {
    const KEY: &str = "DORAMCAP_CRASH_RECORDING";
    if let Some(path) = std::env::var_os(KEY) {
        let path = std::path::PathBuf::from(path);
        let mut writer = writer(create(&path, 1), 1);
        let mut r = writer.reserve_large(meta(1), 20000).unwrap();
        r.put_bytes(7, 20000);
        r.commit().unwrap();
        writer.flush_written().unwrap();
        std::process::exit(0); // simulate process exit before MCAP finalization
    }
    let path = std::env::temp_dir().join(format!("doramcap-crash-{}", std::process::id()));
    assert!(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "abrupt_exit_preserves_complete_record_prefix"])
            .env(KEY, &path)
            .status()
            .unwrap()
            .success()
    );
    let data = std::fs::read(&path).unwrap();
    let scan = |data: &[u8]| {
        let mut cursor = 8;
        let mut messages = 0;
        while cursor + 9 <= data.len() {
            let len = u64::from_le_bytes(data[cursor + 1..cursor + 9].try_into().unwrap()) as usize;
            if len > data.len() - cursor - 9 {
                break;
            }
            if data[cursor] == 5 {
                messages += 1;
            }
            cursor += 9 + len;
        }
        (messages, cursor)
    };
    assert_eq!(scan(&data), (1, data.len()));
    assert_eq!(scan(&data[..data.len() - 1]).0, 0);
    std::fs::remove_file(path).unwrap();
}
