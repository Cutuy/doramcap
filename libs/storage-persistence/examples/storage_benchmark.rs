//! Bounded 64-MiB recording benchmark; startup and readback are outside measurement.
use bytes::BufMut;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    fs::File,
    os::unix::fs::OpenOptionsExt,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
    time::Instant,
};
use storage_persistence::mcap::{Channel, McapWriter, Message, Schema};
struct Alloc;
static ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Alloc {
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
static GLOBAL: Alloc = Alloc;
fn cpu_ns() -> u64 {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut t) },
        0
    );
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn main() {
    for direct in [false, true] {
        let path = std::env::temp_dir().join(format!(
            "doramcap-benchmark-{}-{direct}",
            std::process::id()
        ));
        let file = File::options()
            .write(true)
            .create_new(true)
            .custom_flags(if direct { libc::O_DIRECT } else { 0 })
            .open(&path)
            .unwrap();
        let schema = Schema {
            name: "opaque",
            encoding: "",
            data: b"",
        };
        let channels = [Channel {
            id: 1,
            topic: "benchmark",
            encoding: "",
        }];
        let mut w = if direct {
            McapWriter::direct(file, 32, 65536, schema, &channels, 4096).unwrap()
        } else {
            McapWriter::new(file, 32, 65536, schema, &channels).unwrap()
        };
        let mut durations = Vec::with_capacity(64);
        let cpu = cpu_ns();
        let start = Instant::now();
        ALLOCATIONS.store(0, Relaxed);
        ENABLED.store(true, Relaxed);
        for sequence in 0..64 {
            let t = Instant::now();
            let mut r = w
                .reserve_large(
                    Message {
                        channel: 1,
                        sequence,
                        log_time: sequence as u64,
                        publish_time: sequence as u64,
                    },
                    1 << 20,
                )
                .unwrap();
            r.put_bytes(sequence as u8, 1 << 20);
            r.commit().unwrap();
            w.flush_written().unwrap();
            durations.push(t.elapsed().as_nanos() as u64);
        }
        w.finish().unwrap();
        ENABLED.store(false, Relaxed);
        let elapsed = start.elapsed();
        let cpu = cpu_ns() - cpu;
        let allocations = ALLOCATIONS.load(Relaxed);
        assert_eq!(allocations, 0);
        durations.sort_unstable();
        let data = std::fs::read(&path).unwrap();
        let mut count = 0;
        for msg in mcap::MessageStream::new(&data).unwrap() {
            let msg = msg.unwrap();
            assert_eq!(msg.data.len(), 1 << 20);
            assert!(msg.data.iter().all(|b| *b == msg.sequence as u8));
            count += 1;
        }
        assert_eq!(count, 64);
        println!(
            "mode={} payload_mib=64 elapsed_ms={} throughput_mib_s={:.1} process_cpu_ms={:.1} p99_record_us={:.1} allocations={allocations}",
            if direct { "direct" } else { "buffered" },
            elapsed.as_millis(),
            64.0 / elapsed.as_secs_f64(),
            cpu as f64 / 1e6,
            durations[63] as f64 / 1e3
        );
        std::fs::remove_file(path).unwrap();
    }
}
