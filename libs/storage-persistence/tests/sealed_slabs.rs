mod support;
use std::{
    fs::File,
    io,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use storage_persistence::{
    managed_buffers::BufferPool,
    mcap::{Channel, McapWriter, Message, Schema},
    slabs::SlabPool,
    writer::{Writer, WriterConfig},
};
fn path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("doramcap-slab-{}-{label}", std::process::id()))
}
fn open(path: &Path) -> File {
    File::options()
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_DIRECT)
        .open(path)
        .unwrap()
}
fn message(sequence: u32) -> Message {
    Message {
        channel: 1,
        sequence,
        log_time: 10,
        publish_time: 11,
    }
}
fn mcap(
    file: File,
    pending_capacity: usize,
) -> (McapWriter, storage_persistence::observability::McapMonitor) {
    let (metrics, monitor) = storage_persistence::observability::McapMonitor::channel();
    let writer = McapWriter::new(
        file,
        Schema {
            name: "opaque",
            encoding: "",
            data: b"",
        },
        &[Channel {
            id: 1,
            topic: "slab",
            encoding: "",
        }],
        storage_persistence::mcap::McapConfig {
            metrics,
            buffers: 2,
            buffer_size: 8192,
            alignment: 4096,
            io: WriterConfig {
                pending_capacity,
                max_write_bytes: 4096,
                ..WriterConfig::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    (writer, monitor)
}
#[test]
fn prepared_blocks_share_bytes_across_threads_and_rotation() {
    for size in [4096, 65536] {
        let first = path(&format!("prepared-{size}-a"));
        let second = path(&format!("prepared-{size}-b"));
        let pool = SlabPool::new(1, size, 4096).unwrap();
        let (mut writer, monitor) = mcap(open(&first), 8);
        writer.flush_written().unwrap();
        writer.prepare_slabs(&pool).unwrap();
        let updates = monitor.snapshot().unwrap().writer.registration_updates;
        for iteration in 0..2 {
            let mut small = writer.reserve(message(0), 3).unwrap();
            small.payload_mut().fill(9);
            small.commit().unwrap();
            let builder = writer.prepare_records(pool.acquire().unwrap());
            let block = std::thread::spawn(move || {
                let mut builder = builder;
                builder.reserve(message(1), 7).unwrap().fill(1);
                builder.reserve(message(2), size - 100).unwrap().fill(2);
                builder.seal().unwrap()
            })
            .join()
            .unwrap();
            let reader = block.slab().clone();
            assert_eq!(reader.as_slice().len(), size);
            let expected = reader.as_slice().to_vec();
            let range = writer.append_prepared_records(block).unwrap();
            assert_eq!(range.len, size as u64);
            assert_eq!(
                writer.retire_slabs(&pool).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
            writer.flush_written().unwrap();
            assert_eq!(pool.available(), 0); // reader outlives CQEs
            assert_eq!(reader.as_slice(), expected);
            drop(reader);
            assert_eq!(pool.available(), 1);
            assert_eq!(
                monitor.snapshot().unwrap().writer.registration_updates,
                updates
            );
            let bytes = std::fs::read(if iteration == 0 { &first } else { &second }).unwrap();
            assert_eq!(&bytes[range.offset as usize..][..size], expected);
            if iteration == 0 {
                writer.rotate(open(&second)).unwrap();
            }
        }
        writer.finish().unwrap();
        for file in [first, second] {
            let bytes = std::fs::read(&file).unwrap();
            let messages = mcap::MessageStream::new(&bytes)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(messages.len(), 3);
            let mut messages = messages;
            messages.sort_by_key(|m| m.sequence);
            for (msg, (seq, len, fill)) in
                messages
                    .iter()
                    .zip([(0, 3, 9), (1, 7, 1), (2, size - 100, 2)])
            {
                assert_eq!(msg.sequence, seq);
                assert_eq!(msg.data.len(), len);
                assert!(msg.data.iter().all(|&b| b == fill));
            }
            std::fs::remove_file(file).unwrap();
        }
    }
}
#[test]
fn warmed_reuse_has_zero_updates_and_retirement_keeps_readers_valid() {
    let file = path("reuse");
    let managed = BufferPool::new(1, 4096, 4096).unwrap();
    let pool = SlabPool::new(1, 4096, 4096).unwrap();
    let mut writer = Writer::direct(open(&file), managed, 4096).unwrap();
    writer.prepare_slabs(&pool).unwrap();
    let updates = writer.registration_updates();
    let bytes = writer.registered_bytes();
    for i in 0..1000 {
        let mut slab = pool.acquire().unwrap();
        slab.bytes_mut().fill(i as u8);
        let sealed = slab.seal(4096).unwrap();
        writer.try_append_slab(sealed).unwrap();
        assert_eq!(pool.available(), 0); // no reader, I/O still owns it
        writer.flush_written().unwrap();
        assert_eq!(pool.available(), 1);
        writer.prepare_slabs(&pool).unwrap();
    }
    assert_eq!(writer.registration_updates(), updates);
    assert_eq!(writer.registered_bytes(), bytes);
    let reader = pool.acquire().unwrap().seal(4096).unwrap();
    writer.retire_slabs(&pool).unwrap();
    assert_eq!(reader.as_slice()[0], 999_u32 as u8);
    assert_eq!(writer.registered_bytes(), bytes - 4096);
    writer.prepare_slabs(&pool).unwrap();
    assert_eq!(writer.registration_updates(), updates + 2);
    drop(reader);
    writer.close().unwrap();
    let data = std::fs::read(&file).unwrap();
    for (i, block) in data.as_chunks::<4096>().0.iter().enumerate() {
        assert!(block.iter().all(|&b| b == i as u8));
    }
    std::fs::remove_file(file).unwrap();
}
#[test]
fn rejected_block_preserves_address_and_stream_binding() {
    let a = path("pressure-a");
    let b = path("pressure-b");
    let pool = SlabPool::new(1, 4096, 4096).unwrap();
    let (mut writer, _monitor) = mcap(open(&a), 1);
    let (mut foreign, _foreign_monitor) = mcap(open(&b), 2);
    // The combining message stays buffered until the explicit barrier.
    writer.reserve(message(0), 0).unwrap().commit().unwrap();
    let mut builder = writer.prepare_records(pool.acquire().unwrap());
    assert!(
        builder
            .reserve(
                Message {
                    channel: 9,
                    ..message(1)
                },
                1
            )
            .is_err()
    );
    assert!(builder.reserve(message(1), usize::MAX).is_err());
    builder.reserve(message(1), 5).unwrap().fill(3);
    let block = builder.seal().unwrap();
    let address = block.slab().as_slice().as_ptr();
    let before = foreign.file_bytes();
    let rejected = foreign.append_prepared_records(block).unwrap_err();
    assert_eq!(rejected.error.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(foreign.file_bytes(), before);
    assert_eq!(rejected.value.slab().as_slice().as_ptr(), address);
    writer.flush_written().unwrap();
    writer.append_prepared_records(rejected.value).unwrap();
    writer.finish().unwrap();
    foreign.finish().unwrap();
    assert_eq!(pool.available(), 1);
    let data = std::fs::read(&a).unwrap();
    let messages = mcap::MessageStream::new(&data)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        messages.iter().map(|m| m.sequence).collect::<Vec<_>>(),
        [0, 1]
    );
    for file in [a, b] {
        std::fs::remove_file(file).unwrap();
    }
}
#[test]
fn registration_limit_and_bad_length_return_slab_without_offset_consumption() {
    let file = path("limit");
    let managed = BufferPool::new(1, 4096, 4096).unwrap();
    let pool = SlabPool::new(1, 8192, 4096).unwrap();
    let mut writer = Writer::direct_with_config(
        open(&file),
        managed,
        4096,
        WriterConfig {
            registration_capacity: 1,
            ..WriterConfig::default()
        },
    )
    .unwrap();
    let bad = pool.acquire().unwrap().seal(1).unwrap();
    let error = writer.try_append_slab(bad).unwrap_err();
    assert_eq!(error.error.kind(), io::ErrorKind::InvalidInput);
    drop(error);
    let slab = pool.acquire().unwrap().seal(8192).unwrap();
    let ptr = slab.as_slice().as_ptr();
    let error = writer.try_append_slab(slab).unwrap_err();
    assert_eq!(error.error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(error.value.as_slice().as_ptr(), ptr);
    assert_eq!(writer.accepted_bytes(), 0);
    assert_eq!(writer.registered_ranges(), 1);
    drop(error);
    writer.close().unwrap();
    assert_eq!(pool.available(), 1);
    std::fs::remove_file(file).unwrap();
}

#[test]
fn standalone_paths_do_not_flush_or_reserve_capacity_for_combining() {
    for kind in ["large", "slab", "mapped"] {
        let file = path(kind);
        // Metadata occupies one slot; exactly one slot remains for the new write.
        let (mut writer, monitor) = mcap(open(&file), 2);
        let mut small = writer.reserve(message(0), 3).unwrap();
        small.payload_mut().fill(7);
        small.commit().unwrap();
        let before = monitor.snapshot().unwrap();
        let start = writer.file_bytes() - before.combining_bytes as u64;
        let expected: Vec<u8> = (0..12000)
            .map(|i| ((i * 31 + i / 4096) % 251) as u8)
            .collect();
        match kind {
            "large" => {
                let mut r = writer.reserve(message(1), expected.len()).unwrap();
                r.payload_mut().copy_from_slice(&expected);
                r.try_commit().unwrap();
            }
            "slab" => {
                let pool = SlabPool::new(1, 16384, 4096).unwrap();
                let mut b = writer.prepare_records(pool.acquire().unwrap());
                b.reserve(message(1), expected.len())
                    .unwrap()
                    .copy_from_slice(&expected);
                let range = writer.append_prepared_records(b.seal().unwrap()).unwrap();
                assert_eq!(range.offset, start);
                assert_eq!(range.len, 12288);
            }
            "mapped" => {
                let region = storage_persistence::mapped::MmapRegion::new(
                    memmap2::MmapOptions::new().len(20480).map_anon().unwrap(),
                    Default::default(),
                );
                let mut block = region.acquire(4096..20480).unwrap();
                let len = support::stamp(block.bytes_mut(), message(1), &expected);
                let range = writer
                    .append_external_records(block.seal(len).unwrap())
                    .unwrap();
                assert_eq!(range.offset, start);
                assert_eq!(range.len, 12288);
            }
            _ => unreachable!(),
        }
        let after = monitor.snapshot().unwrap();
        assert_eq!(after.combining_bytes, before.combining_bytes);
        assert_eq!(after.combining_messages, 1);
        assert_eq!(
            writer.file_bytes() - after.combining_bytes as u64 - start,
            12288
        );
        assert!(monitor.snapshot().unwrap().writer.pending <= 2);
        writer.finish().unwrap();
        let bytes = std::fs::read(&file).unwrap();
        let msgs = mcap::MessageStream::new(&bytes)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].sequence, 1);
        assert_eq!(msgs[0].data.as_ref(), expected);
        assert_eq!(msgs[1].sequence, 0);
        assert_eq!(msgs[1].data.as_ref(), [7; 3]);
        std::fs::remove_file(file).unwrap();
    }
}
