#![cfg(feature = "otel")]
use fault_protocol::{AggregationFrame, AggregationPayload};
use fault_watchdog::{
    config::pb::OtelConfig,
    sinks::{Packet, Sink, otel::Logs},
};
use prost::Message;
use std::{
    io::{Read, Write},
    time::Duration,
};
#[test]
fn http_logs_preserve_domain_identity_and_trace() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut block = [0; 4096];
        let (offset, length) = loop {
            let n = stream.read(&mut block).unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&block[..n]);
            if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                let header = std::str::from_utf8(&bytes[..end]).unwrap();
                assert!(header.starts_with("POST /v1/logs "));
                let length = header
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(|v| v.parse::<usize>().unwrap())
                    })
                    .unwrap();
                break (end + 4, length);
            }
        };
        while bytes.len() < offset + length {
            let n = stream.read(&mut block).unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&block[..n]);
        }
        stream.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: application/x-protobuf\r\ncontent-length: 0\r\nconnection: close\r\n\r\n").unwrap();
        opentelemetry_proto::tonic::collector::logs::v1::ExportLogsServiceRequest::decode(
            &bytes[offset..offset + length],
        )
        .unwrap()
    });
    let (r, p) = fault_reporter::Reporter::new(42, Default::default()).unwrap();
    let c = r
        .register(fault_reporter::Definition::new("test", "disk", "write").unwrap())
        .unwrap();
    c.observe(
        fault_reporter::Outcome::Failure,
        fault_reporter::Text::new("disk failed").unwrap(),
        Some(fault_reporter::TraceContext {
            trace_id: [1; 16],
            span_id: [2; 8],
        }),
    )
    .unwrap();
    let mut a =
        fault_aggregation::Aggregator::new(43, Default::default(), Default::default()).unwrap();
    a.reconcile(&p.snapshot(), Duration::ZERO).unwrap();
    let f = AggregationFrame {
        published_unix_ns: Some(1234),
        emitted_at: Duration::ZERO,
        payload: AggregationPayload::Snapshot(a.snapshot()),
    };
    let mut sink = Logs::new(OtelConfig {
        enabled: true,
        endpoint: format!("http://{address}/v1/logs"),
        timeout_ms: 2000,
        service_name: "test-watchdog".into(),
    })
    .unwrap();
    sink.write(&Packet {
        bytes: fault_protocol::encode_aggregation(&f, Default::default()).unwrap(),
    })
    .unwrap();
    Box::new(sink).finish().unwrap();
    let request = server.join().unwrap();
    let record = &request.resource_logs[0].scope_logs[0].log_records[0];
    assert_eq!(record.trace_id, vec![1; 16]);
    assert_eq!(record.span_id, vec![2; 8]);
    assert_eq!(record.time_unix_nano, 1234);
    for key in [
        "fault.source",
        "fault.epoch",
        "fault.revision",
        "fault.health",
        "fault.sequence",
    ] {
        assert!(record.attributes.iter().any(|a| a.key == key));
    }
}
