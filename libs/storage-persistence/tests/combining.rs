use storage_persistence::mcap::{Channel, McapWriter, Message, Schema};
#[test]
fn adjacent_reservations_timer_flush_and_abandonment() {
    let path = std::env::temp_dir().join(format!("doramcap-combine-{}", std::process::id()));
    let file = std::fs::File::options()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    let mut writer = McapWriter::new(
        file,
        2,
        512,
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
    )
    .unwrap();
    let meta = Message {
        channel: 1,
        sequence: 0,
        log_time: 0,
        publish_time: 0,
    };
    let baseline = writer.storage_writes();
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
    assert_eq!(writer.storage_writes(), baseline);
    writer.poll(writer.next_flush_deadline().unwrap()).unwrap();
    assert_eq!(writer.storage_writes(), baseline + 1);
    for _ in 0..32 {
        writer.reserve(meta, 1).unwrap().commit().unwrap();
    }
    writer.flush_written().unwrap();
    assert_eq!(writer.storage_writes(), baseline + 3);
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
