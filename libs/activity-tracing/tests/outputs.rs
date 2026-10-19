use activity_tracing::{BatchView, Config, Result, Runtime, Sink, sinks::McapSink, wire};
use std::sync::{Arc, Mutex};
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
#[test]
fn mcap_preserves_shared_bytes_schema_and_timestamps() {
    let path = std::env::temp_dir().join(format!(
        "activity-{}-{}.mcap",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let sink = McapSink::create(&path).unwrap();
    assert!(McapSink::create(&path).is_err());
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut rt = Runtime::start(
        Config::default(),
        vec![Box::new(sink), Box::new(Collect(shared.clone()))],
    )
    .unwrap();
    let h = rt.handle();
    let reg = h.register_thread("test").unwrap();
    tracing::subscriber::with_default(h.subscriber(), || {
        let span = tracing::info_span!(target:"activity","work");
        let _e = span.enter();
        tracing::info!(target:"activity",value=42);
    });
    drop(reg);
    let report = rt.shutdown().unwrap();
    assert!(
        report
            .sinks
            .iter()
            .all(|r| r.error.is_none() && r.dropped == 0)
    );
    let data = std::fs::read(&path).unwrap();
    let messages = mcap::MessageStream::new(&data)
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    let recorded = shared.lock().unwrap();
    assert_eq!(messages.len(), recorded.len());
    for (message, bytes) in messages.iter().zip(recorded.iter()) {
        assert_eq!(message.data.as_ref(), bytes);
        let b = wire::decode(bytes).unwrap();
        assert_eq!(
            message.log_time,
            b.unix_origin_ns + b.records[0].monotonic_ns
        );
        assert_eq!(
            message.channel.schema.as_ref().unwrap().data.as_ref(),
            wire::DESCRIPTOR
        );
    }
    std::fs::remove_file(path).unwrap();
}
struct Fail;
impl Sink for Fail {
    fn name(&self) -> &str {
        "fail"
    }
    fn write(&mut self, _: BatchView<'_>) -> Result<()> {
        Err("injected failure".into())
    }
}
#[test]
fn failure_is_reported_and_other_output_survives() {
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut rt = Runtime::start(
        Config::default(),
        vec![Box::new(Fail), Box::new(Collect(shared.clone()))],
    )
    .unwrap();
    let h = rt.handle();
    let reg = h.register_thread("test").unwrap();
    tracing::subscriber::with_default(h.subscriber(), || {
        tracing::info!(target:"activity",value=1);
        tracing::info!(target:"activity",value=2);
    });
    drop(reg);
    let report = rt.shutdown().unwrap();
    assert!(report.sinks[0].error.as_ref().unwrap().contains("injected"));
    assert_eq!(shared.lock().unwrap().len(), 2);
    assert_eq!(report.sinks[0].dropped, 2);
}
#[cfg(feature = "websocket")]
#[test]
fn websocket_binary_frame_decodes() {
    use std::{
        net::{TcpListener, TcpStream},
        time::Duration,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let client = std::thread::spawn(move || {
        let stream = TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let (mut ws, _) = tungstenite::client(format!("ws://{addr}/"), stream).unwrap();
        let message = ws.read().unwrap();
        assert!(message.is_binary());
        wire::decode(&message.into_data()).unwrap()
    });
    let (stream, _) = listener.accept().unwrap();
    let sink =
        activity_tracing::sinks::WebSocketSink::accept(stream, Duration::from_secs(2)).unwrap();
    let mut rt = Runtime::start(Config::default(), vec![Box::new(sink)]).unwrap();
    let h = rt.handle();
    let reg = h.register_thread("test").unwrap();
    tracing::subscriber::with_default(
        h.subscriber(),
        || tracing::info!(target:"activity",value=42),
    );
    drop(reg);
    let batch = client.join().unwrap();
    assert_eq!(batch.records.len(), 1);
    let _ = rt.shutdown().unwrap();
}

#[test]
fn slow_sink_drops_independently_without_starving_fast_sink() {
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    struct Slow {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }
    impl Sink for Slow {
        fn name(&self) -> &str {
            "slow"
        }
        fn write(&mut self, _: BatchView<'_>) -> Result<()> {
            let _ = self.entered.send(());
            self.release.recv_timeout(Duration::from_secs(5))?;
            Ok(())
        }
    }
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut rt = Runtime::start(
        Config {
            sink_queue: 1,
            encoded_buffers: 5,
            ..Default::default()
        },
        vec![
            Box::new(Slow {
                entered: entered_tx,
                release: release_rx,
            }),
            Box::new(Collect(shared.clone())),
        ],
    )
    .unwrap();
    let h = rt.handle();
    let reg = h.register_thread("test").unwrap();
    tracing::subscriber::with_default(h.subscriber(), || {
        for n in 0..10 {
            tracing::info!(target:"activity",n);
            if n == 0 {
                entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            let deadline = Instant::now() + Duration::from_secs(5);
            while shared.lock().unwrap().len() < n + 1 {
                assert!(Instant::now() < deadline, "fast sink stalled");
                std::thread::yield_now();
            }
        }
    });
    drop(reg);
    for _ in 0..2 {
        release_tx.send(()).unwrap();
    }
    let report = rt.shutdown().unwrap();
    assert_eq!(report.sinks[0].dropped, 8);
    assert_eq!(report.sinks[1].written, 10);
    assert_eq!(report.sinks[1].dropped, 0);
    assert_eq!(report.capture.encoded_batches, 0);
}
