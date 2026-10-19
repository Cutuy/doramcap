//! Independent MCAP reader and Protobuf validation for demo artifacts.
use prost::Message as _;
fn main() {
    let path = std::env::args().nth(1).expect("capture path");
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.ends_with(b"\x89MCAP0\r\n"));
    let mut counts = [0usize; 5];
    for message in mcap::MessageStream::new(&bytes).unwrap() {
        let message = message.unwrap();
        let packet = telemetry_protocol::pb::Packet::decode(message.data.as_ref()).unwrap();
        if matches!(
            message.channel.topic.as_str(),
            "demo/managed-large" | "demo/sealed-slab"
        ) {
            let Some(telemetry_protocol::pb::packet::Body::Snapshot(frame)) = packet.body else {
                panic!("expected snapshot")
            };
            let prefix = if message.channel.topic == "demo/managed-large" {
                "managed:"
            } else {
                "slab:"
            };
            let payload = frame.source.strip_prefix(prefix).unwrap();
            assert!(payload.bytes().all(|b| b == b'x'));
            assert!(matches!(
                payload.len(),
                262144 | 1048576 | 4194304 | 16777216
            ));
        }
        let channel = match message.channel.topic.as_str() {
            "demo/cpu" => 0,
            "demo/heartbeat" => 1,
            "demo/large-probe" => 2,
            "demo/managed-large" => 3,
            "demo/sealed-slab" => 4,
            topic => panic!("unexpected topic {topic}"),
        };
        counts[channel] += 1;
    }
    assert!(
        counts[..3].iter().all(|n| *n > 0),
        "missing demo channels: {counts:?}"
    );
    if std::env::args().any(|a| a == "--exercise-paths") {
        assert!(
            counts[3] >= 3 && counts[4] >= 10,
            "missing workload records: {counts:?}"
        );
    }
    println!(
        "MCAP readback: CPU={} heartbeat={} large={}",
        counts[0], counts[1], counts[2]
    );
}
