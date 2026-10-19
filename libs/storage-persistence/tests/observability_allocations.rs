use std::{
    alloc::{GlobalAlloc, Layout, System},
    fs::File,
    os::unix::fs::OpenOptionsExt,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
    time::Instant,
};
use storage_persistence::{
    managed_buffers::BufferPool,
    observability::{Collector, EventData, EventKind, Mode, Observer},
    writer::Writer,
};
struct Counting;
static ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if ENABLED.load(Relaxed) {
            ALLOCATIONS.fetch_add(1, Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;
#[test]
fn warmed_collection_is_bounded_and_allocation_free() {
    for mode in [
        None,
        Some(Mode::Off),
        Some(Mode::Summary),
        Some(Mode::Inspect),
        Some(Mode::Capture),
    ] {
        let collector = mode.map(|m| {
            let c =
                Collector::new(128, if m == Mode::Capture { Mode::Inspect } else { m }).unwrap();
            if m == Mode::Capture {
                c.capture(std::time::Duration::from_secs(30), Default::default())
                    .unwrap();
            }
            c
        });
        let observer = collector
            .as_ref()
            .map_or_else(Observer::default, Collector::observer);
        let file = std::env::temp_dir().join(format!(
            "doramcap-observe-alloc-{}-{mode:?}",
            std::process::id()
        ));
        let f = File::options()
            .create_new(true)
            .write(true)
            .custom_flags(libc::O_DIRECT)
            .open(&file)
            .unwrap();
        let pool = BufferPool::new(1, 4096, 4096).unwrap();
        let mut writer = Writer::direct(f, pool.clone(), 4096).unwrap();
        writer.set_observer(observer.clone());
        writer.try_append(pool.reserve(4096).unwrap()).unwrap();
        writer.flush_written().unwrap();
        ALLOCATIONS.store(0, Relaxed);
        ENABLED.store(true, Relaxed);
        let started = Instant::now();
        for _ in 0..1000 {
            writer.try_append(pool.reserve(4096).unwrap()).unwrap();
            writer.flush_written().unwrap();
        }
        let write_time = started.elapsed();
        let started = Instant::now();
        for _ in 0..100_000 {
            std::hint::black_box(&observer).record(EventKind::Accepted, EventData::default);
        }
        let emit_time = started.elapsed();
        ENABLED.store(false, Relaxed);
        assert_eq!(ALLOCATIONS.load(Relaxed), 0);
        println!(
            "mode={mode:?} writer_us_per_4k={:.1} emit_ns={:.1} allocations=0",
            write_time.as_micros() as f64 / 1000.0,
            emit_time.as_nanos() as f64 / 100_000.0
        );
        if let Some(c) = collector {
            assert!(c.read_after(0, usize::MAX).events.len() <= 128);
        }
        writer.close().unwrap();
        std::fs::remove_file(file).unwrap();
    }
}
