use activity_tracing::{BatchView, Config, Result, Runtime, Sink, wire};
use std::sync::{Arc, Mutex};
#[derive(Clone, Default)]
struct Collect(Arc<Mutex<Vec<Vec<u8>>>>);
impl Sink for Collect {
    fn name(&self) -> &str {
        "collect"
    }
    fn write(&mut self, b: BatchView<'_>) -> Result<()> {
        self.0.lock().unwrap().push(b.bytes.to_vec());
        Ok(())
    }
}
fn records(c: &Collect) -> Vec<(wire::proto::Batch, wire::proto::Record)> {
    c.0.lock()
        .unwrap()
        .iter()
        .flat_map(|bytes| {
            let b = wire::decode(bytes).unwrap();
            b.records.clone().into_iter().map(move |r| (b.clone(), r))
        })
        .collect()
}
fn config() -> Config {
    Config {
        blocks: 128,
        encoded_buffers: 128,
        sink_queue: 64,
        records_per_block: 8,
        ..Default::default()
    }
}
#[test]
fn nested_cloned_cross_thread_spans_and_fields() {
    let c = Collect::default();
    let mut rt = Runtime::start(config(), vec![Box::new(c.clone())]).unwrap();
    let h = rt.handle();
    let d = tracing::Dispatch::new(h.subscriber());
    let reg = h.register_thread("main").unwrap();
    tracing::dispatcher::with_default(&d, || {
        let root = tracing::info_span!(target:"activity","root",task.name="camera",answer=tracing::field::Empty);
        root.record("answer", 42u64);
        let entered = root.enter();
        let child = tracing::info_span!(target:"activity","child");
        let clone = child.clone();
        drop(child);
        std::thread::scope(|s| {
            let d = &d;
            let h = &h;
            s.spawn(move||{let _reg=h.register_thread("worker").unwrap();tracing::dispatcher::with_default(d,||{let _e=clone.enter();tracing::info!(target:"activity",value=7u64,text="hello",signed=-4i64,ratio=1.25,yes=true);});drop(clone);});
        });
        drop(entered);
        drop(root);
    });
    drop(reg);
    let report = rt.shutdown().unwrap();
    assert_eq!(report.capture.records, 0);
    assert_eq!(report.sinks[0].dropped, 0);
    let rows = records(&c);
    let starts: Vec<_> = rows.iter().filter(|(_, r)| r.kind == 0).collect();
    assert_eq!(starts.len(), 2);
    let root = &starts[0].1;
    let child = &starts[1].1;
    assert_eq!(child.parent_id, root.span_id);
    assert_eq!(child.trace_root, root.trace_root);
    assert_eq!(
        rows.iter()
            .filter(|(_, r)| r.kind == 4 && r.span_id == child.span_id)
            .count(),
        1
    );
    let (b, event) = rows.iter().find(|(_, r)| r.kind == 1).unwrap();
    assert_eq!(event.span_id, child.span_id);
    assert_ne!(event.tid, root.tid);
    let text = event
        .fields
        .iter()
        .find(|f| b.dictionary[f.name as usize] == "text")
        .unwrap();
    assert_eq!(
        &b.strings[text.offset as usize..(text.offset + text.length) as usize],
        b"hello"
    );
    assert!(rows.iter().any(|(_, r)| r.kind == 2));
}
#[test]
fn partial_flush_and_utf8_limits() {
    let c = Collect::default();
    let mut cfg = config();
    cfg.flush_events = false;
    cfg.string_bytes_per_block = 5;
    let mut rt = Runtime::start(cfg, vec![Box::new(c.clone())]).unwrap();
    let h = rt.handle();
    let reg = h.register_thread("test").unwrap();
    tracing::subscriber::with_default(h.subscriber(), || {
        tracing::info!(target:"activity",text="éééé");
    });
    assert!(c.0.lock().unwrap().is_empty());
    reg.flush();
    drop(reg);
    let report = rt.shutdown().unwrap();
    assert_eq!(report.capture.fields, 1);
    let rows = records(&c);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0.strings, "éé".as_bytes());
}
#[test]
fn shutdown_requires_registration_release() {
    let mut rt = Runtime::start(Config::default(), vec![]).unwrap();
    let h = rt.handle();
    let reg = h.register_thread("test").unwrap();
    assert!(rt.shutdown().is_err());
    drop(reg);
    rt.shutdown().unwrap();
    assert!(h.register_thread("late").is_err());
}
#[test]
fn bounded_span_capacity_and_reuse() {
    let c = Collect::default();
    let mut cfg = config();
    cfg.max_spans = 1;
    let mut rt = Runtime::start(cfg, vec![Box::new(c.clone())]).unwrap();
    let h = rt.handle();
    let reg = h.register_thread("test").unwrap();
    tracing::subscriber::with_default(h.subscriber(), || {
        let a = tracing::info_span!(target:"activity","a");
        let b = tracing::info_span!(target:"activity","b");
        drop(b);
        drop(a);
        let c = tracing::info_span!(target:"activity","c");
        drop(c);
    });
    drop(reg);
    let report = rt.shutdown().unwrap();
    assert_eq!(report.capture.spans, 1);
    assert_eq!(records(&c).iter().filter(|(_, r)| r.kind == 0).count(), 2);
}
#[test]
fn unregistered_capture_is_visible() {
    let mut rt = Runtime::start(Config::default(), vec![]).unwrap();
    tracing::subscriber::with_default(
        rt.handle().subscriber(),
        || tracing::info!(target:"activity",value=1),
    );
    assert_eq!(rt.shutdown().unwrap().capture.unregistered, 1);
}

#[test]
fn async_activity_migrates_with_stable_identity() {
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    use tracing::Instrument;
    let c = Collect::default();
    let mut cfg = config();
    cfg.flush_events = false;
    let mut rt = Runtime::start(cfg, vec![Box::new(c.clone())]).unwrap();
    let h = rt.handle();
    let d = tracing::Dispatch::new(h.subscriber());
    let reg = h.register_thread("origin").unwrap();
    let span = tracing::dispatcher::with_default(
        &d,
        || tracing::info_span!(target:"activity","async-work"),
    );
    let mut polls = 0;
    let mut future = Box::pin(
        std::future::poll_fn(move |_| {
            polls += 1;
            tracing::info!(target:"activity",polls);
            if polls == 1 {
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
        .instrument(span),
    );
    for n in 0..2 {
        std::thread::scope(|s| {
            let future = &mut future;
            let d = &d;
            let h = &h;
            s.spawn(move || {
                let _r = h.register_thread("executor").unwrap();
                let waker = Waker::noop();
                let result = tracing::dispatcher::with_default(d, || {
                    future.as_mut().poll(&mut Context::from_waker(waker))
                });
                assert_eq!(result.is_ready(), n == 1);
            });
        });
    }
    drop(future);
    drop(reg);
    rt.shutdown().unwrap();
    let rows = records(&c);
    let events: Vec<_> = rows.iter().filter(|(_, r)| r.kind == 1).collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].1.span_id, events[1].1.span_id);
    assert_ne!(events[0].1.tid, events[1].1.tid);
    assert_eq!(rows.last().unwrap().1.kind, 4);
}

#[test]
fn thread_registration_capacity_is_enforced_and_reusable() {
    let mut rt = Runtime::start(
        Config {
            max_threads: 1,
            ..Default::default()
        },
        vec![],
    )
    .unwrap();
    let h = rt.handle();
    let registration = h.register_thread("first").unwrap();
    std::thread::scope(|s| {
        let h = &h;
        s.spawn(move || assert!(h.register_thread("second").is_err()));
    });
    drop(registration);
    std::thread::scope(|s| {
        let h = &h;
        s.spawn(move || {
            let _r = h.register_thread("second").unwrap();
        });
    });
    rt.shutdown().unwrap();
}
#[test]
fn context_depth_overflow_recovers_parent() {
    let c = Collect::default();
    let mut cfg = config();
    cfg.max_depth = 1;
    let mut rt = Runtime::start(cfg, vec![Box::new(c.clone())]).unwrap();
    let h = rt.handle();
    let reg = h.register_thread("test").unwrap();
    tracing::subscriber::with_default(h.subscriber(), || {
        let root = tracing::info_span!(target:"activity","root");
        let _r = root.enter();
        {
            let child = tracing::info_span!(target:"activity","child");
            let _c = child.enter();
            tracing::info!(target:"activity",value=1);
        }
        tracing::info!(target:"activity",value=2);
    });
    drop(reg);
    assert_eq!(rt.shutdown().unwrap().capture.spans, 1);
    let rows = records(&c);
    let root = rows.iter().find(|(_, r)| r.kind == 0).unwrap().1.span_id;
    let events: Vec<_> = rows.iter().filter(|(_, r)| r.kind == 1).collect();
    assert_eq!(events[0].1.span_id, 0);
    assert_eq!(events[1].1.span_id, root);
}

#[test]
fn detached_context_preserves_causality_after_publisher_closes() {
    let c = Collect::default();
    let mut rt = Runtime::start(config(), vec![Box::new(c.clone())]).unwrap();
    let h = rt.handle();
    let d = tracing::Dispatch::new(h.subscriber());
    let reg = h.register_thread("publisher").unwrap();
    let context = tracing::dispatcher::with_default(&d, || {
        let publish = tracing::info_span!(target:"activity","publish");
        h.context(&publish).unwrap()
    });
    std::thread::scope(|scope| {
        let h = &h;
        let d = &d;
        scope.spawn(move || {
            let _reg = h.register_thread("subscriber").unwrap();
            tracing::dispatcher::with_default(d, || {
                h.with_parent(context, || {
                    let receive = tracing::info_span!(target:"activity","receive");
                    let _entered = receive.enter();
                    drop(tracing::info_span!(target:"activity","process"));
                })
                .unwrap()
            });
        });
    });
    drop(reg);
    rt.shutdown().unwrap();
    let rows = records(&c);
    let pub_end = rows
        .iter()
        .position(|(_, r)| r.kind == 4 && r.span_id == context.span_id())
        .unwrap();
    let receive = rows
        .iter()
        .position(|(b, r)| r.kind == 0 && b.dictionary[r.name as usize] == "receive")
        .unwrap();
    assert!(pub_end < receive);
    assert_eq!(rows[receive].1.parent_id, context.span_id());
    assert_eq!(rows[receive].1.trace_root, context.trace_root());
    let process = rows
        .iter()
        .find(|(b, r)| r.kind == 0 && b.dictionary[r.name as usize] == "process")
        .unwrap();
    assert_eq!(process.1.parent_id, rows[receive].1.span_id);
}
