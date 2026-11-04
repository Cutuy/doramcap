use activity_tracing::{BatchView, Config, Result, Runtime, Sink, SinkBuilder};
use std::sync::{Arc, Mutex};
#[test]
fn sink_build_failure_fails_start() {
    // Arrange
    struct Refuse;
    impl SinkBuilder for Refuse {
        fn build(self: Box<Self>) -> Result<Box<dyn Sink>> {
            Err("refused".into())
        }
    }
    // Act
    let result = Runtime::start(Config::default(), vec![Box::new(Refuse)]);
    // Assert
    assert_eq!(result.err().unwrap().to_string(), "refused");
}
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
#[cfg(feature = "mcap")]
#[test]
fn mcap_preserves_shared_bytes_schema_and_timestamps() {
    use activity_tracing::{sinks::McapSink, wire};
    let path = std::env::temp_dir().join(format!(
        "activity-{}-{}.mcap",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut rt = Runtime::start(
        Config::default(),
        vec![
            Box::new(McapSink::new(&path)),
            Box::new(Collect(shared.clone())),
        ],
    )
    .unwrap();
    assert!(Runtime::start(Config::default(), vec![Box::new(McapSink::new(&path))]).is_err());
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
    use activity_tracing::wire;
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
