mod support;
use memmap2::MmapOptions;
use std::{
    fs::{File, OpenOptions},
    io,
    os::unix::fs::{FileExt, OpenOptionsExt},
    path::PathBuf,
    ptr::NonNull,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use storage_persistence::{
    managed_buffers::BufferPool,
    mapped::MmapRegion,
    mcap::{Channel, McapWriter, Message, Schema},
    observability::Observer,
    writer::{Writer, WriterConfig},
};
fn page() -> usize {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
}
struct Temp(PathBuf);
impl Temp {
    fn new(shm: bool, label: &str) -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);
        Self(
            PathBuf::from(if shm { "/dev/shm" } else { "/tmp" }).join(format!(
                "mapped-{}-{label}-{}",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            )),
        )
    }
    fn file(&self) -> File {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&self.0)
            .unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn output(path: &Temp, pending: usize) -> Writer {
    Writer::direct_with_config(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_DIRECT)
            .open(&path.0)
            .unwrap(),
        BufferPool::new(1, 4096, 4096).unwrap(),
        4096,
        WriterConfig {
            pending_capacity: pending,
            max_write_bytes: 4096,
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn private_and_shm_ranges_reuse_registration_and_preserve_readers() {
    for private in [true, false] {
        let p = page().max(4096);
        let backing = Temp::new(!private, "ranges");
        let file = backing.file();
        file.set_len((4 * p) as u64).unwrap();
        // SAFETY: unique test file, no truncation or external mutation during mapping.
        let map = unsafe {
            if private {
                MmapOptions::new().map_copy(&file)
            } else {
                MmapOptions::new().map_mut(&file)
            }
        }
        .unwrap();
        let region = MmapRegion::new(map, Observer::default());
        let out = Temp::new(false, "output");
        let mut writer = output(&out, 1);
        writer.prepare_region(&region).unwrap();
        let updates = writer.registration_updates();
        // A header range stays writable while independent block ranges are leased.
        let mut header = region.acquire(0..p).unwrap();
        header.bytes_mut().fill(0xa5);
        let mut expected = Vec::new();
        for generation in 0..32u8 {
            let mut block = region.acquire(p..3 * p).unwrap();
            block.bytes_mut().fill(generation);
            let lease = block.seal(2 * p).unwrap();
            let reader = lease.clone();
            assert!(region.acquire(p..2 * p).is_err());
            writer.try_append_mapped(lease).unwrap();
            assert_eq!(
                writer.retire_region(&region).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
            let other = region.acquire(3 * p..4 * p).unwrap().seal(p).unwrap();
            let rejected = writer.try_append_mapped(other).unwrap_err();
            assert_eq!(rejected.error.kind(), io::ErrorKind::WouldBlock);
            drop(rejected);
            header.bytes_mut()[0] = generation;
            writer.flush_written().unwrap();
            assert!(region.acquire(p..3 * p).is_err());
            assert!(reader.as_slice().iter().all(|b| *b == generation));
            std::thread::spawn(move || drop(reader)).join().unwrap();
            expected.extend(std::iter::repeat_n(generation, 2 * p));
        }
        assert_eq!(writer.registration_updates(), updates);
        assert_eq!(std::fs::read(&out.0).unwrap(), expected);
        drop(header);
        writer.retire_region(&region).unwrap();
        writer.prepare_region(&region).unwrap();
        assert!(writer.registration_updates() > updates);
        let mut bytes = vec![0; 4 * p];
        file.read_exact_at(&mut bytes, 0).unwrap();
        if private {
            assert!(bytes.iter().all(|b| *b == 0));
        } else {
            assert_eq!(bytes[p], 31);
        }
    }
}

#[test]
fn mapped_mcap_batches_bind_format_and_rotate() {
    for private in [true, false] {
        let backing = Temp::new(!private, "mcap-map");
        let file = backing.file();
        file.set_len(65536).unwrap();
        let map = unsafe {
            if private {
                MmapOptions::new().map_copy(&file)
            } else {
                MmapOptions::new().map_mut(&file)
            }
        }
        .unwrap();
        let region = MmapRegion::new(map, Observer::default());
        let out = Temp::new(false, "mcap");
        let next = Temp::new(false, "rotated");
        let open = |p: &Temp| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .custom_flags(libc::O_DIRECT)
                .open(&p.0)
                .unwrap()
        };
        let mut w = McapWriter::new(
            open(&out),
            Schema {
                name: "raw",
                encoding: "",
                data: b"",
            },
            &[Channel {
                id: 1,
                topic: "mapped",
                encoding: "",
            }],
            storage_persistence::mcap::McapConfig {
                buffers: 2,
                buffer_size: 8192,
                alignment: 4096,
                io: WriterConfig {
                    max_write_bytes: 4096,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let wrong_out = Temp::new(false, "wrong-format");
        let mut wrong = McapWriter::new(
            open(&wrong_out),
            Schema {
                name: "other",
                encoding: "",
                data: b"",
            },
            &[Channel {
                id: 2,
                topic: "other",
                encoding: "",
            }],
            storage_persistence::mcap::McapConfig {
                buffers: 2,
                buffer_size: 8192,
                alignment: 4096,
                io: WriterConfig::default(),
                ..Default::default()
            },
        )
        .unwrap();
        for iteration in 0..2 {
            let mut block = region.acquire(4096..32768).unwrap();
            let len = support::stamp(
                block.bytes_mut(),
                Message {
                    channel: 1,
                    sequence: iteration,
                    log_time: 1,
                    publish_time: 1,
                },
                &vec![iteration as u8 + 9; 12000],
            );
            let lease = block.seal(len).unwrap();
            let reader = lease.clone();
            let rejected = wrong.append_external_records(lease).unwrap_err();
            assert_eq!(rejected.error.kind(), io::ErrorKind::InvalidInput);
            assert_eq!(
                rejected.value.as_slice().as_ptr(),
                reader.as_slice().as_ptr()
            );
            let written = w.append_external_records(rejected.value).unwrap();
            w.flush_written().unwrap();
            let file_bytes = std::fs::read(if iteration == 0 { &out.0 } else { &next.0 }).unwrap();
            assert_eq!(
                &file_bytes[written.offset as usize..(written.offset + written.len) as usize],
                reader.as_slice()
            );
            assert!(region.acquire(4096..32768).is_err());
            drop(reader);
            if iteration == 0 {
                w.rotate(open(&next)).unwrap();
            }
        }
        w.finish().unwrap();
        for (i, path) in [&out, &next].iter().enumerate() {
            let bytes = std::fs::read(&path.0).unwrap();
            let msgs = mcap::MessageStream::new(&bytes)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0].data.as_ref(), vec![i as u8 + 9; 12000]);
        }
    }
}

#[derive(Debug)]
struct Signal(Arc<AtomicUsize>);
impl Drop for Signal {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
#[test]
fn imported_range_reuse_token_outlives_io_and_rejections() {
    let p = page().max(4096);
    let mut map = MmapOptions::new().len(3 * p).map_anon().unwrap();
    map.fill(7);
    let dropped = Arc::new(AtomicUsize::new(0));
    let owner = Arc::new((map, Signal(dropped.clone())));
    let weak = Arc::downgrade(&owner);
    let ptr = NonNull::new(owner.0.as_ptr().cast_mut()).unwrap();
    // SAFETY: owner retains mapping; initialized bytes stay immutable throughout.
    let region = unsafe { MmapRegion::import(ptr, 3 * p, owner, Observer::default()) }.unwrap();
    assert!(region.acquire(p..2 * p).is_err());
    let releases = Arc::new(AtomicUsize::new(0));
    let lease = unsafe { region.lease(p..3 * p, 42, Signal(releases.clone())) }.unwrap();
    assert_eq!(lease.generation(), 42);
    let bad = unsafe { region.lease(3 * p..4 * p, 99, Signal(releases.clone())) }.unwrap_err();
    assert_eq!(releases.load(Ordering::SeqCst), 0);
    drop(bad);
    assert_eq!(releases.load(Ordering::SeqCst), 1);
    let out = Temp::new(false, "import");
    let mut w = output(&out, 2);
    w.prepare_region(&region).unwrap();
    let reader = lease.clone();
    w.try_append_mapped(lease).unwrap();
    w.flush_written().unwrap();
    assert_eq!(releases.load(Ordering::SeqCst), 1);
    drop(reader);
    assert_eq!(releases.load(Ordering::SeqCst), 2);
    drop(region);
    assert!(weak.upgrade().is_some());
    drop(w);
    assert!(weak.upgrade().is_none());
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[test]
fn zenoh_posix_blocks_use_provider_guards() {
    use zenoh::{
        Wait,
        shm::{AllocAlignment, ShmProviderBuilder, ZShm},
    };
    let p = page().max(4096);
    let provider = ShmProviderBuilder::default_backend((
        32 * p,
        AllocAlignment::new(p.trailing_zeros() as u8).unwrap(),
    ))
    .wait()
    .unwrap();
    let out = Temp::new(false, "zenoh");
    let mut w = output(&out, 8);
    let mut regions = Vec::new();
    for n in 0..3u8 {
        let mut block = provider
            .alloc((
                2 * p,
                AllocAlignment::new(p.trailing_zeros() as u8).unwrap(),
            ))
            .wait()
            .unwrap();
        block.as_mut().fill(n + 1);
        assert_eq!(block.as_ref().as_ptr() as usize % p, 0);
        let block: ZShm = block.into();
        let ptr = NonNull::new(block.as_ref().as_ptr().cast_mut()).unwrap();
        // SAFETY: immutable Zenoh buffer retains its POSIX segment and allocation.
        let region =
            unsafe { MmapRegion::import(ptr, 2 * p, block.clone(), Observer::default()) }.unwrap();
        w.prepare_region(&region).unwrap();
        let updates = w.registration_updates();
        for generation in 0..8 {
            let lease = unsafe { region.lease(p..2 * p, generation, block.clone()) }.unwrap();
            w.try_append_mapped(lease).unwrap();
            w.flush_written().unwrap();
        }
        assert_eq!(updates, w.registration_updates());
        regions.push(region);
    }
    drop(provider); // Guarded blocks and registration still keep memory valid.
    for r in regions {
        w.retire_region(&r).unwrap();
    }
    let bytes = std::fs::read(&out.0).unwrap();
    for n in 0..3 {
        assert!(
            bytes[n * 8 * p..(n + 1) * 8 * p]
                .iter()
                .all(|b| *b == n as u8 + 1)
        );
    }
}

#[test]
fn mapped_process_producer() {
    if let Some(path) = std::env::var_os("MAPPED_TEST_CHILD") {
        let file = OpenOptions::new().write(true).open(path).unwrap();
        file.write_all_at(&vec![0x6b; 4096], 4096).unwrap();
    }
}
#[test]
fn shm_cross_process_publication() {
    let backing = Temp::new(true, "process");
    let file = backing.file();
    file.set_len(8192).unwrap();
    // Map first; external publication happens before any byte lease is exposed.
    let mut map = unsafe { MmapOptions::new().map_mut(&file) }.unwrap();
    let ptr = NonNull::new(map.as_mut_ptr()).unwrap();
    let region = unsafe { MmapRegion::import(ptr, 8192, map, Observer::default()) }.unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "mapped_process_producer"])
        .env("MAPPED_TEST_CHILD", &backing.0)
        .status()
        .unwrap();
    assert!(status.success());
    // Producer exit is our test-only publication/ownership handoff.
    let lease = unsafe { region.lease(4096..8192, 1, ()) }.unwrap();
    let out = Temp::new(false, "process-out");
    let mut w = output(&out, 1);
    w.try_append_mapped(lease).unwrap();
    w.flush_written().unwrap();
    assert_eq!(std::fs::read(&out.0).unwrap(), vec![0x6b; 4096]);
}

#[test]
fn invalid_ranges_and_alignment_preserve_ownership() {
    let region = MmapRegion::new(
        MmapOptions::new().len(16384).map_anon().unwrap(),
        Observer::default(),
    );
    for range in [0..0, 16384..16385] {
        assert_eq!(
            region.acquire(range).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    let range = region.acquire(0..4096).unwrap();
    let rejected = range.seal(4097).unwrap_err();
    assert!(region.acquire(0..4096).is_err());
    drop(rejected);
    let out = Temp::new(false, "misaligned");
    let mut w = output(&out, 2);
    for (range, len) in [(1..8193, 4096), (0..4096, 4095)] {
        let lease = region.acquire(range.clone()).unwrap().seal(len).unwrap();
        let ptr = lease.as_slice().as_ptr();
        let rejected = w.try_append_mapped(lease).unwrap_err();
        assert_eq!(rejected.error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(rejected.value.as_slice().as_ptr(), ptr);
        assert_eq!(w.accepted_bytes(), 0);
        assert!(region.acquire(range.clone()).is_err());
        drop(rejected);
        assert!(region.acquire(range).is_ok());
    }
}

#[test]
fn existing_payload_uses_prefix_page_without_touching_shm() {
    for private in [true, false] {
        let backing = Temp::new(!private, "external-payload");
        let file = backing.file();
        let size = 4 * 1024 * 1024;
        file.set_len((size + 4096) as u64).unwrap();
        // Caller creates and populates the mapping before giving storage a read lease.
        let mut map = unsafe {
            if private {
                MmapOptions::new().map_copy(&file)
            } else {
                MmapOptions::new().map_mut(&file)
            }
        }
        .unwrap();
        for (i, byte) in map[4096..4096 + size].iter_mut().enumerate() {
            *byte = (i.wrapping_mul(37) % 251) as u8;
        }
        let region = MmapRegion::new(map, Observer::default());
        let releases = Arc::new(AtomicUsize::new(0));
        // Published bytes already exist. No producer mutates this range until token release.
        let lease =
            unsafe { region.lease(4096..4096 + size, 1, Signal(releases.clone())) }.unwrap();
        let reader = lease.clone();
        let pointer = reader.as_slice().as_ptr();
        let expected = reader.as_slice().to_vec();
        let out = Temp::new(false, "external-payload-output");
        let (metrics, monitor) = storage_persistence::observability::McapMonitor::channel();
        let mut w = McapWriter::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .custom_flags(libc::O_DIRECT)
                .open(&out.0)
                .unwrap(),
            Schema {
                name: "raw",
                encoding: "",
                data: b"",
            },
            &[Channel {
                id: 1,
                topic: "payload",
                encoding: "",
            }],
            storage_persistence::mcap::McapConfig {
                metrics,
                buffers: 1,
                buffer_size: 8192,
                alignment: 4096,
                io: Default::default(),
                ..Default::default()
            },
        )
        .unwrap();
        w.flush_written().unwrap();
        // A small managed message stays buffered while the external write is admitted.
        let msg = Message {
            channel: 1,
            sequence: 0,
            log_time: 1,
            publish_time: 1,
        };
        w.reserve(msg, 7).unwrap().commit().unwrap();
        let before = monitor.snapshot().unwrap();
        let range = w
            .append_external_payload(Message { sequence: 1, ..msg }, lease)
            .unwrap();
        assert_eq!(range.len as usize, 4096 + size);
        assert_eq!(monitor.active_writes().count(), 1);
        assert_eq!(
            monitor.snapshot().unwrap().combining_bytes,
            before.combining_bytes
        );
        assert!(region.acquire(4096..4096 + size).is_err());
        w.flush_written().unwrap();
        assert_eq!(reader.as_slice().as_ptr(), pointer);
        assert_eq!(reader.as_slice(), expected);
        // Reusing the same mapping and prefix allocation performs no registration updates.
        let updates = monitor.snapshot().unwrap().writer.registration_updates;
        let queued = monitor
            .snapshot()
            .unwrap()
            .writer
            .counters
            .queued_operations;
        w.append_external_payload(Message { sequence: 2, ..msg }, reader.clone())
            .unwrap();
        w.flush_written().unwrap();
        assert_eq!(
            monitor.snapshot().unwrap().writer.registration_updates,
            updates
        );
        assert_eq!(
            monitor
                .snapshot()
                .unwrap()
                .writer
                .counters
                .queued_operations
                - queued,
            2
        );
        w.finish().unwrap();
        let bytes = std::fs::read(&out.0).unwrap();
        let start = range.offset as usize;
        assert_eq!(bytes[start], 0x80);
        assert_eq!(bytes[start + 4096 - 31], 5);
        assert_eq!(&bytes[start + 4096..start + 4096 + size], expected);
        let messages = mcap::MessageStream::new(&bytes)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let external = messages
            .iter()
            .filter(|m| m.sequence > 0)
            .collect::<Vec<_>>();
        assert_eq!(external.len(), 2);
        for m in external {
            assert_eq!(m.data.as_ref(), expected);
        }
        assert!(region.acquire(4096..4096 + size).is_err());
        assert_eq!(releases.load(Ordering::SeqCst), 0);
        drop(reader);
        assert_eq!(releases.load(Ordering::SeqCst), 1);
        assert!(region.acquire(4096..4096 + size).is_ok());
    }
}

#[test]
fn external_mcap_admission_rejects_bad_layouts_intact() {
    let region = MmapRegion::new(
        MmapOptions::new().len(16384).map_anon().unwrap(),
        Observer::default(),
    );
    let out = Temp::new(false, "invalid-external-mcap");
    let mut w = McapWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_DIRECT)
            .open(&out.0)
            .unwrap(),
        Schema {
            name: "raw",
            encoding: "",
            data: b"",
        },
        &[Channel {
            id: 1,
            topic: "raw",
            encoding: "",
        }],
        storage_persistence::mcap::McapConfig {
            buffers: 2,
            buffer_size: 8192,
            alignment: 4096,
            io: Default::default(),
            ..Default::default()
        },
    )
    .unwrap();
    let msg = Message {
        channel: 1,
        sequence: 1,
        log_time: 1,
        publish_time: 1,
    };
    for (offset, len) in [(1, 4096), (0, 4095)] {
        let lease = region
            .acquire(offset..offset + len)
            .unwrap()
            .seal(len)
            .unwrap();
        let ptr = lease.as_slice().as_ptr();
        let before = w.file_bytes();
        let rejected = w.append_external_payload(msg, lease).unwrap_err();
        assert_eq!(rejected.error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(rejected.value.as_slice().as_ptr(), ptr);
        assert_eq!(w.file_bytes(), before);
        drop(rejected);
    }
    for bad in [0, 1, 2, 3] {
        let mut block = region.acquire(0..8192).unwrap();
        let len = support::stamp(block.bytes_mut(), msg, &[77; 4096]);
        match bad {
            0 => block.bytes_mut()[0] = 2, // externally supplied footer
            1 => block.bytes_mut()[9..11].copy_from_slice(&99_u16.to_le_bytes()),
            2 => block.bytes_mut()[1..9].copy_from_slice(&u64::MAX.to_le_bytes()),
            _ => block.bytes_mut()[1..9].copy_from_slice(&1_u64.to_le_bytes()),
        }
        let lease = block.seal(len).unwrap();
        let ptr = lease.as_slice().as_ptr();
        let before = w.file_bytes();
        let rejected = w.append_external_records(lease).unwrap_err();
        assert_eq!(rejected.error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(rejected.value.as_slice().as_ptr(), ptr);
        assert_eq!(w.file_bytes(), before);
        drop(rejected);
    }
    w.finish().unwrap();
}
