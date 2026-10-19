use std::{fs::File, io, os::unix::fs::OpenOptionsExt, path::PathBuf};
use storage_persistence::{
    managed_buffers::{BufferPool, DEFAULT_BUFFER_SIZE, PoolError},
    mcap::{Channel, McapWriter, Message, Schema},
    writer::{Writer, WriterConfig},
};
fn path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("doramcap-growable-{}-{label}", std::process::id()))
}
fn open(path: &std::path::Path, alignment: usize) -> File {
    File::options()
        .write(true)
        .create_new(true)
        .custom_flags(if alignment > 1 { libc::O_DIRECT } else { 0 })
        .open(path)
        .unwrap()
}
fn meta(sequence: u32) -> Message {
    Message {
        channel: 1,
        sequence,
        log_time: 1,
        publish_time: 2,
    }
}
fn mcap(file: File, alignment: usize, config: WriterConfig) -> McapWriter {
    McapWriter::with_config(
        file,
        8,
        DEFAULT_BUFFER_SIZE,
        Schema {
            name: "bytes",
            encoding: "",
            data: b"",
        },
        &[Channel {
            id: 1,
            topic: "bytes",
            encoding: "",
        }],
        alignment,
        config,
    )
    .unwrap()
}
#[test]
fn growth_reuse_and_explicit_trim() {
    let pool = BufferPool::new(8, DEFAULT_BUFFER_SIZE, 4096).unwrap();
    let mut initial: Vec<_> = (0..8)
        .map(|_| pool.reserve(DEFAULT_BUFFER_SIZE).unwrap())
        .collect();
    for (i, b) in initial.iter_mut().enumerate() {
        b.as_mut_slice().fill(i as u8);
    }
    let addresses: Vec<_> = initial.iter().map(|b| b.as_slice().as_ptr()).collect();
    let medium = pool.reserve(7 * DEFAULT_BUFFER_SIZE).unwrap();
    let huge = pool.reserve(100 * 1024 * 1024).unwrap();
    assert_eq!(huge.as_slice().len(), 100 * 1024 * 1024);
    assert_eq!(huge.as_slice().as_ptr() as usize % 4096, 0);
    assert_eq!(pool.capacity(), 10);
    assert_eq!(pool.available(), 0);
    assert!(matches!(
        pool.reserve(usize::MAX),
        Err(PoolError::InvalidLayout)
    ));
    assert!(matches!(pool.trim(), Err(PoolError::Busy)));
    for (i, b) in initial.iter().enumerate() {
        assert_eq!(b.as_slice().as_ptr(), addresses[i]);
        assert!(b.as_slice().iter().all(|x| *x == i as u8));
    }
    let medium_ptr = medium.as_slice().as_ptr();
    drop(medium);
    drop(huge);
    let smaller = pool.reserve(6 * DEFAULT_BUFFER_SIZE).unwrap();
    assert_eq!(smaller.as_slice().as_ptr(), medium_ptr); // best fit, not 100 MiB
    drop(smaller);
    drop(initial);
    let bytes = pool.allocated_bytes();
    assert_eq!(pool.free_bytes(), bytes);
    assert_eq!(pool.trim().unwrap(), bytes);
    assert_eq!(pool.capacity(), 0);
    assert_eq!(pool.allocated_bytes(), 0);
    drop(pool.reserve(1).unwrap());
    assert_eq!(pool.allocated_bytes(), DEFAULT_BUFFER_SIZE);
}
#[test]
fn registration_growth_backpressure_and_split_writes() {
    let path = path("admission");
    let pool = BufferPool::new(1, 4096, 4096).unwrap();
    let config = WriterConfig {
        queue_depth: 1,
        pending_capacity: 2,
        registration_capacity: 3,
        max_write_bytes: 4096,
    };
    let mut writer =
        Writer::direct_with_config(open(&path, 4096), pool.clone(), 4096, config).unwrap();
    let mut first = pool.reserve(8192).unwrap();
    first.as_mut_slice().fill(1);
    let mut second = pool.reserve(12288).unwrap();
    second.as_mut_slice().fill(2);
    writer.prepare(&first).unwrap();
    writer.prepare(&second).unwrap();
    let registered = writer.registered_bytes();
    writer.try_append(first).unwrap();
    writer.try_append(second).unwrap();
    let third = pool.reserve(4096).unwrap();
    let address = third.as_slice().as_ptr();
    let error = writer.try_append(third).unwrap_err();
    assert_eq!(error.error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(error.lease.as_slice().as_ptr(), address);
    assert_eq!(writer.accepted_bytes(), 20480);
    assert_eq!(writer.written_bytes(), 0); // no CQEs processed to satisfy any reservation
    assert!(writer.trim(&pool).is_err());
    writer.flush_written().unwrap();
    assert_eq!(pool.available(), 2); // rejected lease still belongs to caller
    writer.try_append(error.lease).unwrap();
    writer.flush_written().unwrap();
    assert_eq!(writer.registered_bytes(), registered);
    #[cfg(feature = "diagnostics")]
    assert_eq!(writer.submitted_operations(), 6);
    writer.sync().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes[..8192].iter().all(|b| *b == 1));
    assert!(bytes[8192..20480].iter().all(|b| *b == 2));
    let retained = pool.allocated_bytes();
    assert_eq!(writer.trim(&pool).unwrap(), retained);
    assert_eq!(writer.registered_bytes(), 0);
    let lease = pool.reserve(4096).unwrap();
    writer.prepare(&lease).unwrap();
    assert_eq!(writer.registered_ranges(), 1);
    writer.append(lease).unwrap();
    writer.close().unwrap();
    std::fs::remove_file(path).unwrap();
}
#[test]
fn registration_limit_keeps_existing_writes_and_returns_new_lease() {
    let path = path("register-limit");
    let pool = BufferPool::new(1, 4096, 4096).unwrap();
    let mut writer = Writer::direct_with_config(
        open(&path, 4096),
        pool.clone(),
        4096,
        WriterConfig {
            registration_capacity: 1,
            ..WriterConfig::default()
        },
    )
    .unwrap();
    writer.append(pool.reserve(4096).unwrap()).unwrap();
    let lease = pool.reserve(8192).unwrap();
    let ptr = lease.as_slice().as_ptr();
    let failure = writer.try_append(lease).unwrap_err();
    assert_eq!(failure.error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(failure.lease.as_slice().as_ptr(), ptr);
    assert_eq!(writer.accepted_bytes(), 4096);
    assert_eq!(writer.registered_ranges(), 1);
    drop(failure);
    writer.close().unwrap();
    std::fs::remove_file(path).unwrap();
}
#[test]
fn contiguous_hundred_mib_records_without_draining_and_exact_readback() {
    large_readback(&[1], 100 * 1024 * 1024);
}

#[test]
fn contiguous_direct_records_without_draining_and_exact_readback() {
    large_readback(&[512, 4096], 16 * 1024 * 1024);
}

#[test]
#[ignore = "requires more than 104 MiB RLIMIT_MEMLOCK; run explicitly on qualified hosts"]
fn hundred_mib_registered_direct_readback() {
    large_readback(&[512, 4096], 100 * 1024 * 1024);
}

fn large_readback(alignments: &[usize], huge_size: usize) {
    for &alignment in alignments {
        let path = path(&format!("huge-{alignment}-{huge_size}"));
        let mut writer = mcap(
            open(&path, alignment),
            alignment,
            WriterConfig {
                queue_depth: 1,
                max_write_bytes: 1024 * 1024,
                ..WriterConfig::default()
            },
        );
        for (seq, size) in [(0, 7 * DEFAULT_BUFFER_SIZE), (1, huge_size)] {
            let before = writer.pending_writes();
            assert!(before > 0);
            let mut reservation = writer.reserve(meta(seq), size).unwrap();
            let ptr = reservation.payload().as_ptr();
            let slice = reservation.payload_mut();
            assert_eq!(slice.len(), size);
            for (i, b) in slice.iter_mut().enumerate() {
                *b = (i % 251) as u8;
            }
            assert_eq!(reservation.payload().as_ptr(), ptr);
            reservation.commit().unwrap();
            assert_eq!(writer.pending_writes(), before + 1); // every earlier write still pending
            #[cfg(feature = "diagnostics")]
            assert_eq!(writer.diagnostics().completions, 0);
        }
        writer.finish().unwrap();
        let data = std::fs::read(&path).unwrap();
        let messages = mcap::MessageStream::new(&data)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(messages.len(), 2);
        for (msg, size) in messages.iter().zip([7 * DEFAULT_BUFFER_SIZE, huge_size]) {
            assert_eq!(msg.data.len(), size);
            assert!(
                msg.data
                    .iter()
                    .enumerate()
                    .all(|(i, b)| *b == (i % 251) as u8)
            );
        }
        std::fs::remove_file(path).unwrap();
    }
}
#[test]
fn mcap_commit_backpressure_retains_contiguous_bytes() {
    let path = path("retry");
    let mut writer = mcap(
        open(&path, 1),
        1,
        WriterConfig {
            pending_capacity: 1,
            queue_depth: 1,
            ..WriterConfig::default()
        },
    );
    // Metadata occupies the only pending slot. Reservation must still succeed.
    let mut reservation = writer.reserve(meta(0), DEFAULT_BUFFER_SIZE * 7).unwrap();
    reservation.payload_mut().fill(9);
    let ptr = reservation.payload().as_ptr();
    assert_eq!(
        reservation.try_commit().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(reservation.payload().as_ptr(), ptr);
    reservation.progress(true).unwrap();
    reservation.try_commit().unwrap();
    drop(reservation);
    writer.finish().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let msg = mcap::MessageStream::new(&bytes)
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    assert!(msg.data.iter().all(|b| *b == 9));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn reservation_succeeds_with_full_pending_queue_and_active_combiner() {
    let path = path("active-retry");
    let mut writer = mcap(
        open(&path, 1),
        1,
        WriterConfig {
            pending_capacity: 1,
            queue_depth: 1,
            ..WriterConfig::default()
        },
    );
    writer.reserve(meta(0), 1).unwrap().commit().unwrap(); // active small record, metadata still pending
    let mut large = writer.reserve(meta(1), 7 * DEFAULT_BUFFER_SIZE).unwrap();
    large.payload_mut().fill(42);
    assert_eq!(
        large.try_commit().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    large.progress(true).unwrap();
    // Small batch takes the only pending slot; large lease remains retryable.
    assert_eq!(
        large.try_commit().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    large.progress(true).unwrap();
    large.try_commit().unwrap();
    drop(large);
    writer.finish().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let messages = mcap::MessageStream::new(&bytes)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].sequence, 0);
    assert_eq!(messages[1].sequence, 1);
    assert!(messages[1].data.iter().all(|b| *b == 42));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn framing_boundaries_and_small_batching() {
    for alignment in [1, 512, 4096] {
        let path = path(&format!("boundaries-{alignment}"));
        let mut writer = mcap(open(&path, alignment), alignment, WriterConfig::default());
        writer.flush_written().unwrap();
        let initial_writes = writer.storage_writes();
        for seq in 0..10 {
            let mut r = writer.reserve(meta(seq), 7).unwrap();
            r.payload_mut().fill(seq as u8);
            r.commit().unwrap();
        }
        assert_eq!(writer.storage_writes(), initial_writes);
        writer.flush_written().unwrap();
        assert_eq!(writer.storage_writes(), initial_writes + 1);
        let inline = writer.max_inline_payload();
        let sizes = [
            inline - 1,
            inline,
            inline + 1,
            DEFAULT_BUFFER_SIZE - 1,
            DEFAULT_BUFFER_SIZE,
            DEFAULT_BUFFER_SIZE + 1,
        ];
        for (i, size) in sizes.into_iter().enumerate() {
            let mut r = writer.reserve(meta(i as u32 + 10), size).unwrap();
            r.payload_mut().fill((i + 10) as u8);
            r.commit().unwrap();
        }
        writer.finish().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let messages = mcap::MessageStream::new(&bytes)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(messages.len(), 16);
        for (i, message) in messages.iter().enumerate() {
            assert_eq!(message.data.len(), if i < 10 { 7 } else { sizes[i - 10] });
            assert!(message.data.iter().all(|b| *b == i as u8));
        }
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn expired_combiner_poll_drives_a_full_pending_queue() {
    let path = path("poll-pressure");
    let mut writer = mcap(
        open(&path, 1),
        1,
        WriterConfig {
            pending_capacity: 1,
            queue_depth: 1,
            ..WriterConfig::default()
        },
    );
    let mut r = writer.reserve(meta(0), 7).unwrap();
    r.payload_mut().fill(42);
    r.commit().unwrap();
    let deadline = writer.next_flush_deadline().unwrap();
    // Metadata fills admission; the expired batch must stay intact while CQEs progress.
    for _ in 0..10_000 {
        writer.poll(deadline).unwrap();
        if writer.next_flush_deadline().is_none() && writer.pending_writes() == 0 {
            break;
        }
        std::thread::yield_now();
    }
    assert!(writer.next_flush_deadline().is_none());
    assert_eq!(writer.pending_writes(), 0);
    writer.finish().unwrap();
    let data = std::fs::read(&path).unwrap();
    let messages = mcap::MessageStream::new(&data)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(&*messages[0].data, &[42; 7]);
    std::fs::remove_file(path).unwrap();
}
