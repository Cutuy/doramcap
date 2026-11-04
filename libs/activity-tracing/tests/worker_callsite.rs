use activity_tracing::{BatchView, Config, Result, Runtime, Sink, wire};
use std::sync::{Arc, Mutex, mpsc};

fn shared_storage_operation() -> tracing::Span {
    tracing::info_span!(target: "activity::storage", "shared_storage_operation")
}

struct Recorder {
    batches: Arc<Mutex<Vec<Vec<u8>>>>,
    first_write: Option<mpsc::Sender<()>>,
}
impl Sink for Recorder {
    fn name(&self) -> &str {
        "recorder"
    }
    fn write(&mut self, batch: BatchView<'_>) -> Result<()> {
        // Recording uses the same instrumented operation as the application.
        assert!(shared_storage_operation().is_disabled());
        self.batches.lock().unwrap().push(batch.bytes.to_vec());
        if let Some(first_write) = self.first_write.take() {
            first_write.send(())?;
        }
        Ok(())
    }
}

// This integration-test executable contains one test so other subscribers cannot
// change tracing's process-wide callsite registration before this scenario.
#[test]
fn recorder_first_use_preserves_application_spans_and_stays_silent() {
    // Arrange
    let batches = Arc::new(Mutex::new(Vec::new()));
    let (first_write, recorded) = mpsc::channel();
    let mut runtime = Runtime::start(
        Config::default(),
        vec![Box::new(Recorder {
            batches: batches.clone(),
            first_write: Some(first_write),
        })],
    )
    .unwrap();
    let handle = runtime.handle();
    let registration = handle.register_thread("application").unwrap();

    tracing::subscriber::with_default(handle.subscriber(), || {
        // Act: force the recorder to use the shared trace point first.
        tracing::info!(target: "activity", "start recording");
        recorded
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();

        // Act / Assert: the application's later use must still create a span.
        let span = shared_storage_operation();
        assert!(
            !span.is_disabled(),
            "recorder disabled the application span"
        );
        drop(span);
    });
    drop(registration);
    let report = runtime.shutdown().unwrap();

    // Assert: exactly the application's span was recorded, with no worker loss.
    let mut starts = 0;
    let mut ends = 0;
    for bytes in batches.lock().unwrap().iter() {
        let batch = wire::decode(bytes).unwrap();
        for record in &batch.records {
            if batch.dictionary[record.name as usize] == "shared_storage_operation" {
                starts += usize::from(record.kind == 0);
                ends += usize::from(record.kind == 4);
            }
        }
    }
    assert_eq!((starts, ends), (1, 1));
    assert_eq!(report.capture.unregistered, 0);
    assert_eq!(report.capture.records, 0);
    assert_eq!(report.sinks[0].dropped, 0);
    assert!(report.sinks[0].error.is_none());
}
