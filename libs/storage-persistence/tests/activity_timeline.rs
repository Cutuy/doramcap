#![cfg(feature = "activity-tracing")]
use activity_tracing::{BatchView, Config, Runtime, Sink, wire};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    os::unix::fs::OpenOptionsExt,
    sync::{Arc, Mutex},
};
use storage_persistence::{
    mapped::MmapRegion,
    mcap::{Channel, McapWriter, Message, Schema},
    slabs::SlabPool,
    writer::WriterConfig,
};
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<Vec<u8>>>>);
impl Sink for Capture {
    fn name(&self) -> &str {
        "test"
    }
    fn write(&mut self, b: BatchView<'_>) -> activity_tracing::Result<()> {
        self.0.lock().unwrap().push(b.bytes.to_vec());
        Ok(())
    }
}
fn number(b: &wire::proto::Batch, r: &wire::proto::Record, key: &str) -> u64 {
    r.fields
        .iter()
        .find(|f| b.dictionary[f.name as usize] == key)
        .unwrap()
        .bits
}
fn string(b: &wire::proto::Batch, r: &wire::proto::Record, key: &str) -> String {
    let f = r
        .fields
        .iter()
        .find(|f| b.dictionary[f.name as usize] == key)
        .unwrap();
    String::from_utf8(b.strings[f.offset as usize..(f.offset + f.length) as usize].to_vec())
        .unwrap()
}
#[test]
fn activity_capture_correlates_every_path_and_split_io() {
    let capture = Capture::default();
    let mut runtime = Runtime::start(
        Config {
            blocks: 128,
            encoded_buffers: 128,
            sink_queue: 64,
            ..Default::default()
        },
        vec![Box::new(capture.clone())],
    )
    .unwrap();
    let h = runtime.handle();
    let registration = h.register_thread("writer-test").unwrap();
    let path = std::env::temp_dir().join(format!("storage-timeline-{}.mcap", std::process::id()));
    tracing::subscriber::with_default(h.subscriber(), || {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_DIRECT)
            .open(&path)
            .unwrap();
        let mut w = McapWriter::with_config(
            file,
            2,
            8192,
            Schema {
                name: "raw",
                encoding: "",
                data: b"",
            },
            &[Channel {
                id: 1,
                topic: "test",
                encoding: "",
            }],
            4096,
            WriterConfig {
                max_write_bytes: 4096,
                ..Default::default()
            },
        )
        .unwrap();
        let msg = |sequence| Message {
            channel: 1,
            sequence,
            log_time: 0,
            publish_time: 0,
        };
        let mut small = w.reserve(msg(0), 17).unwrap();
        small.payload_mut().fill(3);
        small.commit().unwrap();
        let mut large = w.reserve(msg(1), 12000).unwrap();
        large.payload_mut().fill(4);
        large.commit().unwrap();
        let pool = SlabPool::new(1, 8192, 4096).unwrap();
        let mut slab = w.prepare_records(pool.acquire().unwrap());
        slab.reserve(msg(2), 7000).unwrap().fill(5);
        w.append_prepared_records(slab.seal().unwrap()).unwrap();
        let region = MmapRegion::new(
            memmap2::MmapOptions::new().len(16384).map_anon().unwrap(),
            Default::default(),
        );
        let mut mapped = w.prepare_mapped_records(region.acquire(4096..16384).unwrap());
        mapped.reserve(msg(3), 7000).unwrap().fill(6);
        w.append_mapped_records(mapped.seal().unwrap()).unwrap();
        assert_eq!(w.snapshot().combining_messages, 1);
        w.finish().unwrap();
    });
    drop(registration);
    let report = runtime.shutdown().unwrap();
    assert_eq!(report.capture.records, 0);
    assert_eq!(report.capture.fields, 0);
    assert_eq!(report.sinks[0].dropped, 0);
    assert!(report.sinks[0].error.is_none());
    let batches: Vec<_> = capture
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|b| wire::decode(b).unwrap())
        .collect();
    let rows: Vec<_> = batches
        .iter()
        .flat_map(|b| b.records.iter().map(move |r| (b, r)))
        .collect();
    let starts: BTreeMap<_, _> = rows
        .iter()
        .filter(|(_, r)| r.kind == 0)
        .map(|(b, r)| (r.span_id, (*b, *r)))
        .collect();
    let ends: BTreeMap<_, _> = rows
        .iter()
        .filter(|(_, r)| r.kind == 4)
        .map(|(_, r)| (r.span_id, r.monotonic_ns))
        .collect();
    assert_eq!(starts.len(), ends.len());
    let mut paths = BTreeSet::new();
    let mut combined = 0;
    let mut combining = 0;
    let mut mapped_time = 0;
    for (&id, &(b, r)) in &starts {
        assert!(ends[&id] >= r.monotonic_ns);
        match b.dictionary[r.name as usize].as_str() {
            "storage.combining" => combining = id,
            "storage.write" => {
                let path = string(b, r, "path");
                if path == "combined" {
                    combined = id;
                }
                if path == "mapped" {
                    mapped_time = r.monotonic_ns;
                }
                paths.insert(path);
                assert_eq!(number(b, r, "direct"), 1);
                let mut ops: Vec<_> = starts.values().filter(|(_, o)| o.parent_id == id).collect();
                ops.sort_by_key(|(b, o)| number(b, o, "offset"));
                let mut offset = number(b, r, "offset");
                for (b, o) in ops {
                    assert_eq!(number(b, o, "offset"), offset);
                    let len = number(b, o, "len");
                    assert!(len <= 4096);
                    offset += len;
                    assert!(ends[&o.span_id] <= ends[&id]);
                    assert!(rows.iter().any(|(b, e)| e.kind == 1
                        && e.span_id == o.span_id
                        && number(b, e, "result") == len));
                }
                assert_eq!(offset, number(b, r, "offset") + number(b, r, "len"));
            }
            "storage.io" => assert!(starts.contains_key(&r.parent_id)),
            other => panic!("unexpected span {other}"),
        }
    }
    assert_eq!(
        paths,
        ["managed", "combined", "large", "slab", "mapped"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
    assert!(combined != 0 && combining != 0);
    assert!(
        rows.iter()
            .any(|(_, r)| r.kind == 3 && r.span_id == combined && r.linked_id == combining)
    );
    assert!(starts[&combined].1.monotonic_ns >= mapped_time);
    assert!(starts[&combining].1.monotonic_ns <= mapped_time);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn failed_and_cancelled_writes_close_their_spans() {
    use storage_persistence::{managed_buffers::BufferPool, writer::Writer};
    let capture = Capture::default();
    let mut runtime = Runtime::start(Config::default(), vec![Box::new(capture.clone())]).unwrap();
    let h = runtime.handle();
    let registration = h.register_thread("failure-test").unwrap();
    let path =
        std::env::temp_dir().join(format!("storage-timeline-failure-{}", std::process::id()));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    tracing::subscriber::with_default(h.subscriber(), || {
        let pool = BufferPool::new(2, 512, 512).unwrap();
        let mut failed =
            Writer::new(OpenOptions::new().read(true).open(&path).unwrap(), 2).unwrap();
        failed.append(pool.reserve(512).unwrap()).unwrap();
        assert!(failed.flush_written().is_err());
        let mut cancelled = Writer::new(file, 2).unwrap();
        cancelled.append(pool.reserve(512).unwrap()).unwrap();
        cancelled.cancel_and_drain().unwrap();
        assert_eq!(pool.available(), 2);
    });
    drop(registration);
    let report = runtime.shutdown().unwrap();
    assert_eq!(report.capture.records, 0);
    assert_eq!(report.sinks[0].dropped, 0);
    let batches: Vec<_> = capture
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|b| wire::decode(b).unwrap())
        .collect();
    let rows: Vec<_> = batches
        .iter()
        .flat_map(|b| b.records.iter().map(move |r| (b, r)))
        .collect();
    let starts: BTreeSet<_> = rows
        .iter()
        .filter(|(_, r)| r.kind == 0)
        .map(|(_, r)| r.span_id)
        .collect();
    let ends: BTreeSet<_> = rows
        .iter()
        .filter(|(_, r)| r.kind == 4)
        .map(|(_, r)| r.span_id)
        .collect();
    assert_eq!(starts, ends);
    let outcomes: Vec<_> = rows
        .iter()
        .filter(|(b, r)| {
            r.kind == 1
                && r.fields
                    .iter()
                    .any(|f| b.dictionary[f.name as usize] == "outcome")
        })
        .map(|(b, r)| string(b, r, "outcome"))
        .collect();
    assert!(outcomes.iter().any(|s| s.starts_with("Failed(")));
    assert!(outcomes.iter().any(|s| s == "Cancelled"));
    std::fs::remove_file(path).unwrap();
}
