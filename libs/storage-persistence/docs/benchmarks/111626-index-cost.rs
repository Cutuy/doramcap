//! Base/current cost sampler for the three existing MCAP paths.
//! Workloads, measurements and reproduction: 111626-index-cost.md.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    io,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::{Duration, Instant},
};
use storage_persistence::{
    mcap::{Channel, McapConfig, McapWriter, Message, Schema},
    slabs::SlabPool,
    writer::WriterConfig,
};
struct Counting;
static ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if ENABLED.load(Ordering::Relaxed) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if ENABLED.load(Ordering::Relaxed) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static GLOBAL: Counting = Counting;
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
fn main() -> io::Result<()> {
    let indexed = std::env::args().nth(1).as_deref() == Some("current");
    for path in ["combine", "large", "slab"] {
        for load in ["low", "burst", "saturated"] {
            if std::env::args().nth(2).as_deref() == Some("combine-saturated")
                && (path != "combine" || load != "saturated")
            {
                continue;
            }
            let size: usize = match path {
                "combine" => 256,
                "large" => 1024 * 1024 + 7,
                _ => 64 * 1024 + 7,
            };
            let count = match load {
                "low" => 8,
                "burst" => 64,
                _ => match path {
                    "combine" => 8192,
                    "large" => 128,
                    _ => 512,
                },
            };
            let file = std::env::temp_dir()
                .join(format!("mcap-cost-{}-{path}-{load}", std::process::id()));
            let mut w = McapWriter::create(
                &file,
                Schema {
                    name: "opaque",
                    encoding: "",
                    data: b"",
                },
                &[
                    Channel {
                        id: 1,
                        topic: "one",
                        encoding: "",
                    },
                    Channel {
                        id: 2,
                        topic: "two",
                        encoding: "",
                    },
                ],
                McapConfig {
                    buffers: 4,
                    buffer_size: 65536,
                    io: WriterConfig {
                        pending_capacity: 16,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )?;
            let slab_bytes = (size + 49 + 31 + 9).next_multiple_of(4096);
            let pool = SlabPool::new(4, slab_bytes, 4096)?;
            if path == "slab" {
                w.prepare_slabs(&pool)?;
            }
            let meta = Message {
                channel: 1,
                sequence: 0,
                log_time: 1,
                publish_time: 1,
            };
            drop(w.reserve(meta, size)?);
            w.flush_written()?;
            let mut durations = Vec::with_capacity(count);
            ALLOCS.store(0, Ordering::Relaxed);
            ENABLED.store(true, Ordering::Relaxed);
            let start = Instant::now();
            let cpu_start = cpu_ns();
            for sequence in 0..count {
                let t = Instant::now();
                let message = Message {
                    channel: if sequence % 2 == 0 { 1 } else { 2 },
                    sequence: sequence as u32,
                    log_time: sequence as u64 + 1,
                    publish_time: sequence as u64 + 1,
                };
                if path == "slab" {
                    let slab = loop {
                        match pool.acquire() {
                            Ok(s) => break s,
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => w.flush_written()?,
                            Err(e) => return Err(e),
                        }
                    };
                    let mut builder = w.prepare_records(slab);
                    builder.reserve(message, size)?.fill(sequence as u8);
                    let mut block = builder.seal().map_err(|e| e.error)?;
                    loop {
                        match w.append_prepared_records(block) {
                            Ok(_) => break,
                            Err(e) if e.error.kind() == io::ErrorKind::WouldBlock => {
                                block = e.value;
                                w.flush_written()?;
                            }
                            Err(e) => return Err(e.error),
                        }
                    }
                } else {
                    loop {
                        let accepted = {
                            match w.reserve(message, size) {
                                Ok(mut r) => {
                                    r.payload_mut().fill(sequence as u8);
                                    loop {
                                        match r.try_commit() {
                                            Ok(()) => break,
                                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                                r.progress(true)?
                                            }
                                            Err(e) => return Err(e),
                                        }
                                    }
                                    true
                                }
                                Err(e) if e.kind() == io::ErrorKind::WouldBlock => false,
                                Err(e) => return Err(e),
                            }
                        };
                        if accepted {
                            break;
                        }
                        w.flush_written()?;
                    }
                }
                if load == "low" || (load == "burst" && (sequence + 1) % 16 == 0) {
                    w.flush_written()?;
                }
                durations.push(t.elapsed().as_nanos() as u64);
                if load == "low" {
                    std::thread::sleep(Duration::from_millis(20));
                }
                if load == "burst" && (sequence + 1) % 16 == 0 && sequence + 1 < count {
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
            w.finish()?;
            let wall = start.elapsed();
            let cpu = cpu_ns() - cpu_start;
            ENABLED.store(false, Ordering::Relaxed);
            let allocations = ALLOCS.load(Ordering::Relaxed);
            durations.sort_unstable();
            let p99 = durations[(durations.len() * 99 / 100).min(durations.len() - 1)];
            let file_bytes = std::fs::metadata(&file)?.len();
            {
                use std::io::{Read, Seek, SeekFrom};
                let mut f = std::fs::File::open(&file)?;
                f.seek(SeekFrom::Start(4096))?;
                let mut opcode = [0];
                f.read_exact(&mut opcode)?;
                assert_eq!(
                    opcode[0],
                    if indexed { 6 } else { 5 },
                    "benchmark library provenance"
                );
            }
            let (mut chunks, mut message_indexes, mut data_bytes) = (0_u64, 0_u64, 0_u64);
            {
                use std::io::{Read, Seek, SeekFrom};
                let mut f = std::fs::File::open(&file)?;
                let mut offset = 8_u64;
                while offset + 9 <= file_bytes - 8 {
                    f.seek(SeekFrom::Start(offset))?;
                    let mut header = [0; 9];
                    f.read_exact(&mut header)?;
                    let body = u64::from_le_bytes(header[1..9].try_into().unwrap());
                    if header[0] == 6 {
                        chunks += 1;
                    }
                    if header[0] == 7 {
                        message_indexes += 1;
                    }
                    if header[0] == 15 {
                        data_bytes = offset - 4096;
                        break;
                    }
                    offset += 9 + body;
                }
            }
            let summary_bytes = file_bytes - 4096 - data_bytes - 50;
            std::fs::remove_file(file)?;
            println!(
                "path={path} load={load} messages={count} payload_bytes={size} file_bytes={file_bytes} data_bytes={data_bytes} chunks={chunks} message_indexes={message_indexes} summary_bytes={summary_bytes} wall_ms={:.3} cpu_ms={:.3} p99_us={:.3} allocations={allocations}",
                wall.as_secs_f64() * 1e3,
                cpu as f64 / 1e6,
                p99 as f64 / 1e3
            );
        }
    }
    Ok(())
}
