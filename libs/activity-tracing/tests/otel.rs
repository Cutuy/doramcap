#![cfg(feature = "otel")]
use activity_tracing::{BatchView, Config, Result, Runtime, Sink, sinks::OtelSink, wire};
use opentelemetry_sdk::{
    error::OTelSdkResult,
    trace::{SpanData, SpanExporter},
};
use std::sync::{Arc, Mutex, atomic::Ordering};
#[derive(Clone, Default, Debug)]
struct Export(Arc<Mutex<Vec<SpanData>>>);
impl SpanExporter for Export {
    async fn export(&self, b: Vec<SpanData>) -> OTelSdkResult {
        self.0.lock().unwrap().extend(b);
        Ok(())
    }
}
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<wire::proto::Batch>>>);
impl Sink for Capture {
    fn name(&self) -> &str {
        "capture"
    }
    fn write(&mut self, b: BatchView<'_>) -> Result<()> {
        self.0.lock().unwrap().push(wire::decode(b.bytes)?);
        Ok(())
    }
}
#[test]
fn exported_ids_parent_events_and_links_match_recording() {
    let export = Export::default();
    let recorded = Capture::default();
    let sink = OtelSink::new(export.clone(), 16).unwrap();
    let stats = sink.stats();
    let mut rt = Runtime::start(
        Config {
            encoded_buffers: 64,
            sink_queue: 16,
            ..Default::default()
        },
        vec![Box::new(sink), Box::new(recorded.clone())],
    )
    .unwrap();
    let h = rt.handle();
    let reg = h.register_thread("test").unwrap();
    tracing::subscriber::with_default(h.subscriber(), || {
        let root = tracing::info_span!(target:"activity","root");
        let _r = root.enter();
        let child = tracing::info_span!(target:"activity","child",answer=tracing::field::Empty);
        child.follows_from(root.id());
        child.record("answer", 42u64);
        let _c = child.enter();
        tracing::info!(target:"activity",milestone="ready");
    });
    drop(reg);
    let report = rt.shutdown().unwrap();
    assert!(
        report
            .sinks
            .iter()
            .all(|s| s.error.is_none() && s.dropped == 0)
    );
    let exported = export.0.lock().unwrap();
    assert_eq!(exported.len(), 2);
    let child = exported.iter().find(|s| s.name == "child").unwrap();
    let root = exported.iter().find(|s| s.name == "root").unwrap();
    assert_eq!(child.parent_span_id, root.span_context.span_id());
    assert_eq!(child.events.len(), 1);
    assert_eq!(child.links.len(), 1);
    assert!(child.attributes.iter().any(|a| a.key.as_str() == "answer"));
    let batches = recorded.0.lock().unwrap();
    let (b, r) = batches
        .iter()
        .find_map(|b| {
            b.records
                .iter()
                .find(|r| r.kind == 0 && b.dictionary[r.name as usize] == "child")
                .map(|r| (b, r))
        })
        .unwrap();
    assert_eq!(
        child.span_context.span_id().to_bytes(),
        r.span_id.to_be_bytes()
    );
    let mut trace = [0; 16];
    trace[..8].copy_from_slice(&b.session.to_be_bytes());
    trace[8..].copy_from_slice(&r.trace_root.to_be_bytes());
    assert_eq!(child.span_context.trace_id().to_bytes(), trace);
    assert_eq!(stats.incomplete.load(Ordering::Relaxed), 0);
}
#[test]
fn unfinished_and_evicted_spans_are_explicit() {
    let export = Export::default();
    let sink = OtelSink::new(export.clone(), 1).unwrap();
    let stats = sink.stats();
    let mut rt = Runtime::start(Config::default(), vec![Box::new(sink)]).unwrap();
    let h = rt.handle();
    let reg = h.register_thread("test").unwrap();
    let (a, b) = tracing::subscriber::with_default(h.subscriber(), || {
        (
            tracing::info_span!(target:"activity","a"),
            tracing::info_span!(target:"activity","b"),
        )
    });
    drop(reg);
    rt.shutdown().unwrap();
    drop((a, b));
    assert_eq!(stats.incomplete.load(Ordering::Relaxed), 2);
    assert!(export.0.lock().unwrap().iter().all(|s| {
        s.attributes
            .iter()
            .any(|a| a.key.as_str() == "activity.incomplete")
    }));
}

#[test]
fn otlp_http_reaches_local_collector() {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
        time::Duration,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut first = String::new();
        reader.read_line(&mut first).unwrap();
        assert!(first.starts_with("POST /v1/traces "));
        let mut length = 0;
        let mut protobuf = false;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            let line = line.to_ascii_lowercase();
            if let Some(value) = line.strip_prefix("content-length:") {
                length = value.trim().parse::<usize>().unwrap();
            }
            if line.starts_with("content-type:") && line.contains("application/x-protobuf") {
                protobuf = true;
            }
        }
        assert!(protobuf && length > 0);
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes).unwrap();
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    });
    let sink = OtelSink::http(
        format!("http://{addr}/v1/traces"),
        Duration::from_secs(3),
        16,
    )
    .unwrap();
    let mut rt = Runtime::start(Config::default(), vec![Box::new(sink)]).unwrap();
    let h = rt.handle();
    let reg = h.register_thread("test").unwrap();
    tracing::subscriber::with_default(h.subscriber(), || {
        drop(tracing::info_span!(target:"activity","http-span"));
    });
    drop(reg);
    let report = rt.shutdown().unwrap();
    assert!(report.sinks[0].error.is_none(), "{:?}", report.sinks);
    server.join().unwrap();
}
