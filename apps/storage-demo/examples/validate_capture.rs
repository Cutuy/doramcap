//! Independent MCAP reader and Protobuf validation for demo artifacts.
use prost::Message as _;
fn main() {
    let path = std::env::args().nth(1).expect("capture path");
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.ends_with(b"\x89MCAP0\r\n"));
    let mut counts = [0usize; 3];
    for message in mcap::MessageStream::new(&bytes).unwrap() {
        let message = message.unwrap();
        telemetry_protocol::pb::Packet::decode(message.data.as_ref()).unwrap();
        let channel = match message.channel.topic.as_str() {
            "demo/cpu" => 0,
            "demo/heartbeat" => 1,
            "demo/large-probe" => 2,
            topic => panic!("unexpected topic {topic}"),
        };
        counts[channel] += 1;
    }
    assert!(
        counts.iter().all(|n| *n > 0),
        "missing demo channels: {counts:?}"
    );
    println!(
        "MCAP readback: CPU={} heartbeat={} large={}",
        counts[0], counts[1], counts[2]
    );
}
