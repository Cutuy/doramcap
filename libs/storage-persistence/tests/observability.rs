use std::{
    fs::File,
    io,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::Duration,
};
use storage_persistence::{
    managed_buffers::BufferPool,
    mcap::{Channel, McapWriter, Message, Schema},
    observability::{CaptureFilter, Collector, EventKind, Mode},
    slabs::{SlabPhase, SlabPool},
    writer::{Writer, WriterConfig},
};
fn path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "doramcap-observation-{}-{label}",
        std::process::id()
    ))
}
fn open(path: &Path) -> File {
    File::options()
        .create_new(true)
        .write(true)
        .custom_flags(libc::O_DIRECT)
        .open(path)
        .unwrap()
}
fn message(sequence: u32) -> Message {
    Message {
        channel: 1,
        sequence,
        log_time: 0,
        publish_time: 0,
    }
}
fn mcap(
    file: File,
    capacity: usize,
    observer: storage_persistence::observability::Observer,
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
            topic: "test",
            encoding: "",
        }],
        storage_persistence::mcap::McapConfig {
            metrics,
            observer,
            buffers: 2,
            buffer_size: 8192,
            alignment: 4096,
            io: WriterConfig {
                pending_capacity: capacity,
                max_write_bytes: 4096,
                ..Default::default()
            },
        },
    )
    .unwrap();
    (writer, monitor)
}
#[test]
fn lifecycle_correlation_split_io_and_reader_retention() {
    let file = path("lifecycle");
    let collector = Collector::new(128, Mode::Inspect).unwrap();
    let pool = SlabPool::with_observer(1, 8192, 4096, collector.observer()).unwrap();
    let (mut writer, monitor) = mcap(open(&file), 8, collector.observer());
    writer.flush_written().unwrap();
    writer.prepare_slabs(&pool).unwrap();
    let mut builder = writer.prepare_records(pool.acquire().unwrap());
    assert_eq!(pool.snapshot().slabs[0].phase, SlabPhase::Writable);
    builder.reserve(message(1), 7000).unwrap().fill(17);
    let block = builder.seal().unwrap();
    let reader = block.slab().clone();
    let slab_use = pool.snapshot().slabs[0].slab_use_id;
    let before = monitor.snapshot().unwrap();
    writer.append_prepared_records(block).unwrap();
    // Submission can already retire I/O; correlate through retained history.
    let accepted = collector
        .read_after(0, 128)
        .events
        .into_iter()
        .find(|e| e.kind == EventKind::Accepted && e.data.ids.slab_use == slab_use)
        .unwrap();
    assert_eq!(pool.snapshot().slabs[0].readers, 1);
    writer.flush_written().unwrap();
    let after = monitor.snapshot().unwrap();
    assert_eq!(after.writer.pending, 0);
    assert_eq!(
        after.writer.counters.queued_operations - before.writer.counters.queued_operations,
        2
    );
    assert_eq!(
        after.writer.counters.observed_completions - before.writer.counters.observed_completions,
        2
    );
    assert_eq!(after.writer.counters.short_retries, 0); // splitting is distinct from a short write
    assert!(pool.snapshot().slabs[0].retained_after_io());
    let history = collector.read_after(0, 128);
    let events: Vec<_> = history
        .events
        .iter()
        .filter(|e| e.data.ids.write == accepted.data.ids.write)
        .collect();
    assert_eq!(
        events
            .iter()
            .filter(|e| e.kind == EventKind::Accepted)
            .count(),
        1
    );
    let queued: Vec<_> = events
        .iter()
        .filter(|e| e.kind == EventKind::SqeQueued)
        .collect();
    let completed: Vec<_> = events
        .iter()
        .filter(|e| e.kind == EventKind::CqeObserved)
        .collect();
    assert_eq!(queued.len(), 2);
    for (q, c) in queued.iter().zip(&completed) {
        assert_eq!(q.data.ids.operation, c.data.ids.operation);
        assert_eq!(q.data.ids.region, accepted.data.ids.region);
        assert!(c.data.duration_us.is_some());
    }
    assert_ne!(queued[0].data.ids.operation, queued[1].data.ids.operation);
    assert!(events.iter().any(|e| e.kind == EventKind::Written));
    std::thread::spawn(move || drop(reader)).join().unwrap();
    assert_eq!(pool.available(), 1); // collector/history owns no lease
    let new = pool.acquire().unwrap();
    let reused = pool.snapshot().slabs[0];
    assert_eq!(reused.region_id, accepted.data.ids.region);
    assert_ne!(reused.slab_use_id, accepted.data.ids.slab_use);
    drop(new);
    writer.finish().unwrap();
    assert!(
        collector
            .read_after(0, 128)
            .events
            .iter()
            .any(|e| e.kind == EventKind::SyncCompleted)
    );
    std::fs::remove_file(file).unwrap();
}
#[test]
fn snapshots_survive_loss_late_attachment_and_runtime_modes() {
    let file = path("late");
    let pool = BufferPool::new(2, 4096, 4096).unwrap();
    let mut writer = Writer::direct(open(&file), pool.clone(), 4096).unwrap();
    writer.try_append(pool.reserve(4096).unwrap()).unwrap();
    let old = writer.active_writes().next().unwrap();
    assert!(old.age_us.is_none());
    let collector = Collector::new(2, Mode::Summary).unwrap();
    writer.set_observer(collector.observer());
    writer.try_append(pool.reserve(4096).unwrap()).unwrap();
    assert_eq!(writer.active_writes().count(), 2);
    assert!(writer.active_writes().nth(1).unwrap().age_us.is_some());
    assert!(collector.read_after(0, 10).events.is_empty());
    collector.set_mode(Mode::Inspect).unwrap();
    writer.flush_written().unwrap();
    let history = collector.read_after(0, 10);
    assert!(history.overwritten_total > 0);
    assert!(history.missed_before_cursor > 0);
    let snapshot = writer.snapshot();
    assert_eq!(
        (
            snapshot.pending,
            snapshot.counters.accepted,
            snapshot.counters.written
        ),
        (0, 2, 2)
    );
    collector
        .capture(
            Duration::from_secs(10),
            CaptureFilter {
                writer: Some(writer.writer_id()),
                ..Default::default()
            },
        )
        .unwrap();
    writer.sync().unwrap();
    assert_eq!(writer.snapshot().synced_bytes, 8192);
    collector.set_mode(Mode::Off).unwrap();
    let cursor = collector.read_after(0, 10).events.last().unwrap().sequence;
    writer.close().unwrap();
    assert!(collector.read_after(cursor, 10).events.is_empty());
    std::fs::remove_file(file).unwrap();
}
#[test]
fn rejection_combining_and_rotation_are_visible() {
    let first = path("rotate-a");
    let second = path("rotate-b");
    let collector = Collector::new(128, Mode::Inspect).unwrap();
    let slabs = SlabPool::with_observer(1, 4096, 4096, collector.observer()).unwrap();
    let (mut writer, monitor) = mcap(open(&first), 1, collector.observer());
    writer.reserve(message(1), 5).unwrap().commit().unwrap();
    let mut builder = writer.prepare_records(slabs.acquire().unwrap());
    builder.reserve(message(2), 5).unwrap().fill(2);
    let rejected = writer
        .append_prepared_records(builder.seal().unwrap())
        .unwrap_err();
    assert_eq!(rejected.error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(
        monitor
            .snapshot()
            .unwrap()
            .writer
            .counters
            .rejected_capacity,
        1
    );
    assert_eq!(monitor.snapshot().unwrap().combining_messages, 1);
    assert_eq!(monitor.snapshot().unwrap().combining_bytes, 36);
    writer.flush_written().unwrap();
    assert_eq!(monitor.snapshot().unwrap().combining_messages, 0);
    writer.append_prepared_records(rejected.value).unwrap();
    writer.flush_written().unwrap();
    let old = monitor.snapshot().unwrap().writer;
    writer.rotate(open(&second)).unwrap();
    let new = monitor.snapshot().unwrap().writer;
    assert_eq!(old.writer_id, new.writer_id);
    assert_ne!(old.file_id, new.file_id);
    assert_eq!(new.synced_bytes, 0);
    let h = collector.read_after(0, 128);
    for kind in [
        EventKind::CombiningFlushed,
        EventKind::Rejected(io::ErrorKind::WouldBlock),
        EventKind::FileRotated,
        EventKind::SyncCompleted,
    ] {
        assert!(h.events.iter().any(|e| e.kind == kind), "missing {kind:?}");
    }
    writer.finish().unwrap();
    for file in [first, second] {
        std::fs::remove_file(file).unwrap();
    }
}
#[test]
fn cancellation_and_pool_retirement_preserve_counters_and_memory() {
    let file = path("cancel");
    let collector = Collector::new(64, Mode::Inspect).unwrap();
    let pool = SlabPool::with_observer(2, 8192, 4096, collector.observer()).unwrap();
    let managed = BufferPool::new(1, 4096, 4096).unwrap();
    let mut writer = Writer::direct(open(&file), managed, 4096).unwrap();
    writer.set_observer(collector.observer());
    writer.prepare_slabs(&pool).unwrap();
    for _ in 0..2 {
        writer
            .try_append_slab(pool.acquire().unwrap().seal(8192).unwrap())
            .unwrap();
    }
    writer.cancel_and_drain().unwrap(); // both accepted but not yet queued
    let snapshot = writer.snapshot();
    assert_eq!(
        (
            snapshot.counters.accepted,
            snapshot.counters.cancelled,
            snapshot.pending
        ),
        (2, 2, 0)
    );
    assert_eq!(pool.available(), 2);
    writer.retire_slabs(&pool).unwrap();
    let events = collector.read_after(0, 64).events;
    assert_eq!(
        events
            .iter()
            .filter(|e| e.kind == EventKind::Cancelled)
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| e.kind == EventKind::RegistrationRetired)
            .count(),
        2
    );
    drop(pool);
    drop(writer);
    assert!(!collector.read_after(0, 64).events.is_empty());
    std::fs::remove_file(file).unwrap();
}

#[test]
fn injected_monitor_isolated_and_retains_final_state() {
    let first = path("monitor-a");
    let second = path("monitor-b");
    let collector = Collector::new(128, Mode::Inspect).unwrap();
    let (mut a, a_monitor) = mcap(open(&first), 8, collector.observer());
    let (b, b_monitor) = mcap(open(&second), 8, Default::default());
    let b_initial = b_monitor.snapshot().unwrap();
    assert_ne!(
        a_monitor.snapshot().unwrap().writer.writer_id,
        b_initial.writer.writer_id
    );
    assert!(
        collector
            .read_after(0, 128)
            .events
            .iter()
            .any(|e| e.kind == EventKind::Registered)
    );
    a.reserve(message(1), 5).unwrap().commit().unwrap();
    assert_eq!(a_monitor.snapshot().unwrap().combining_messages, 1);
    assert_eq!(b_monitor.snapshot().unwrap().combining_messages, 0);
    a.finish().unwrap();
    let final_state = a_monitor.snapshot().unwrap();
    assert_eq!(final_state.combining_messages, 0);
    assert_eq!(final_state.writer.pending, 0);
    assert!(final_state.writer.synced_bytes > 0);
    assert_eq!(final_state.writer.counters.syncs, 1);
    assert_eq!(a_monitor.active_writes().count(), 0);
    assert_eq!(
        b_monitor.snapshot().unwrap().writer.counters.accepted,
        b_initial.writer.counters.accepted
    );
    b.finish().unwrap();
    std::fs::remove_file(first).unwrap();
    std::fs::remove_file(second).unwrap();
}
