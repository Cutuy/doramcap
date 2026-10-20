use storage_persistence::mcap::{Channel, McapWriter, Message, Schema};
#[test]
fn adjacent_reservations_timer_flush_and_abandonment() {
    let path = std::env::temp_dir().join(format!("doramcap-combine-{}", std::process::id()));
    let file = std::fs::File::options()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    let (metrics, monitor) = storage_persistence::observability::McapMonitor::channel();
    let mut writer = McapWriter::new(
        file,
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
        storage_persistence::mcap::McapConfig {
            metrics,
            buffers: 2,
            buffer_size: 512,
            alignment: 1,
            io: Default::default(),
            ..Default::default()
        },
    )
    .unwrap();
    let meta = Message {
        channel: 1,
        sequence: 0,
        log_time: 0,
        publish_time: 0,
    };
    let baseline = monitor.snapshot().unwrap().writer.counters.accepted;
    let mut first = writer.reserve(meta, 1).unwrap();
    first.payload_mut()[0] = 7;
    let ptr = first.payload().as_ptr() as usize;
    first.commit().unwrap();
    {
        let mut discarded = writer.reserve(meta, 10).unwrap();
        discarded.payload_mut().fill(0xff);
    }
    let mut next = writer.reserve(meta, 1).unwrap();
    assert_eq!(next.payload().as_ptr() as usize, ptr + 32);
    next.payload_mut()[0] = 8;
    next.commit().unwrap();
    assert_eq!(
        monitor.snapshot().unwrap().writer.counters.accepted,
        baseline
    );
    writer.poll(writer.next_flush_deadline().unwrap()).unwrap();
    assert_eq!(
        monitor.snapshot().unwrap().writer.counters.accepted,
        baseline + 1
    );
    for _ in 0..32 {
        writer.reserve(meta, 1).unwrap().commit().unwrap();
    }
    writer.flush_written().unwrap();
    assert_eq!(
        monitor.snapshot().unwrap().writer.counters.accepted,
        baseline + 3
    );
    writer.finish().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let messages = mcap::MessageStream::new(&bytes)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(messages.len(), 34);
    assert_eq!(messages[0].data.as_ref(), &[7]);
    assert_eq!(messages[1].data.as_ref(), &[8]);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn submissions_flush_aged_batches_and_reclaim_without_polling() {
    use std::time::{Duration, Instant};
    let path = std::env::temp_dir().join(format!("doramcap-submission-{}", std::process::id()));
    let file = std::fs::File::options()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    let (metrics, monitor) = storage_persistence::observability::McapMonitor::channel();
    let mut writer = McapWriter::new(
        file,
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
        storage_persistence::mcap::McapConfig {
            metrics,
            buffers: 2,
            buffer_size: 512,
            alignment: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = Message {
        channel: 1,
        sequence: 0,
        log_time: 0,
        publish_time: 0,
    };
    writer.reserve(meta, 1).unwrap().commit().unwrap();
    let baseline = monitor.snapshot().unwrap().writer.counters.accepted;
    std::thread::sleep(Duration::from_millis(110));
    // A new reservation flushes the expired batch without an explicit poll.
    writer.reserve(meta, 1).unwrap().commit().unwrap();
    assert!(monitor.snapshot().unwrap().writer.counters.accepted > baseline);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut count = 2;
    while count < 200 {
        match writer.reserve(meta, 1) {
            Ok(reservation) => {
                reservation.commit().unwrap();
                count += 1;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => panic!("{error}"),
        }
        assert!(
            Instant::now() < deadline,
            "submissions failed to reclaim capacity"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(monitor.snapshot().unwrap().writer.counters.accepted > baseline + 2);
    writer.finish().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let messages = mcap::MessageStream::new(&bytes)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(messages.len(), count);
    std::fs::remove_file(path).unwrap();
}
