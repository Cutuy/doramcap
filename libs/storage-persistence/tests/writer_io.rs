use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
struct CountAlloc;
static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for CountAlloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if COUNTING.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static GLOBAL: CountAlloc = CountAlloc;

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};
use storage_persistence::{managed_buffers::BufferPool, writer::Writer};
#[test]
fn real_ring_readback_errors_cancellation_and_drop() {
    let path = std::env::temp_dir().join(format!("doramcap-ring-{}", std::process::id()));
    let file = File::options()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let append_file = File::options().append(true).open(&path).unwrap();
    assert!(Writer::new(append_file, 1).is_err());
    let pool = BufferPool::new(4, 4096, 4096).unwrap();
    let mut writer = Writer::new(file.try_clone().unwrap(), 4).unwrap();
    assert!(!writer.sqpoll_enabled());
    for i in 0..4 {
        let mut lease = pool.reserve(4096).unwrap();
        lease.as_mut_slice().fill(i);
        assert_eq!(writer.append(lease).unwrap().offset, u64::from(i) * 4096);
    }
    assert_eq!(pool.available(), 0);
    writer.sync().unwrap();
    assert_eq!(writer.written_bytes(), 16384);
    assert_eq!(pool.available(), 4);
    let mut read = file;
    read.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    read.read_to_end(&mut bytes).unwrap();
    for (i, part) in bytes.as_chunks::<4096>().0.iter().enumerate() {
        assert!(part.iter().all(|b| *b == i as u8));
    }
    COUNTING.store(true, Relaxed);
    for _ in 0..1000 {
        writer.append(pool.reserve(4096).unwrap()).unwrap();
        writer.flush_written().unwrap();
    }
    COUNTING.store(false, Relaxed);
    assert_eq!(ALLOCS.load(Relaxed), 0);
    writer.append(pool.reserve(4096).unwrap()).unwrap();
    drop(writer); // pending work drained
    assert_eq!(pool.available(), 4);
    let mut writer = Writer::new(read.try_clone().unwrap(), 4).unwrap();
    writer.append(pool.reserve(4096).unwrap()).unwrap();
    let _ = writer.cancel_and_drain(); // either write wins or ECANCELED
    assert_eq!(writer.outstanding(), 0);
    assert_eq!(pool.available(), 4);
    assert!(writer.append(pool.reserve(1).unwrap()).is_err());
    drop(writer);
    let mut bad = Writer::new(File::open(&path).unwrap(), 1).unwrap();
    bad.append(pool.reserve(1).unwrap()).unwrap();
    assert!(bad.flush_written().is_err());
    assert_eq!(pool.available(), 4);
    assert!(bad.close().is_err());
    std::fs::remove_file(path).unwrap();
}
