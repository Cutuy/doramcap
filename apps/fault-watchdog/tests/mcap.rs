#![cfg(feature = "mcap")]
use fault_protocol::{AggregationFrame, AggregationPayload};
use fault_watchdog::{
    config::pb::McapConfig,
    sinks::{Packet, Sink, mcap::Recording},
};
use std::time::Duration;
#[test]
fn descriptor_readback_flush_rotation_and_finalization() {
    let directory = std::env::temp_dir().join(format!(
        "fault-mcap-{}",
        fault_watchdog::incarnation().unwrap()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("faults.mcap");
    let mut sink = Recording::new(
        McapConfig {
            enabled: true,
            path: path.to_str().unwrap().into(),
            rotation_bytes: 1,
            flush_ms: 1,
        },
        1024 * 1024,
    )
    .unwrap();
    let frame = AggregationFrame {
        published_unix_ns: Some(1234),
        emitted_at: Duration::ZERO,
        payload: AggregationPayload::Snapshot(fault_aggregation::View {
            epoch: 1,
            revision: 0,
            stats: Default::default(),
            sources: vec![],
        }),
    };
    let packet = Packet {
        bytes: fault_protocol::encode_aggregation(&frame, Default::default()).unwrap(),
    };
    sink.write(&packet).unwrap();
    std::thread::sleep(Duration::from_millis(2));
    sink.tick().unwrap();
    sink.write(&packet).unwrap();
    Box::new(sink).finish().unwrap();
    let mut count = 0;
    for entry in std::fs::read_dir(&directory).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        for message in mcap::MessageStream::new(&bytes).unwrap() {
            let message = message.unwrap();
            assert_eq!(message.channel.topic, "faults");
            let schema = message.channel.schema.as_ref().unwrap();
            assert_eq!(schema.name, fault_protocol::AGGREGATION_SCHEMA_NAME);
            assert_eq!(schema.data.as_ref(), fault_protocol::FILE_DESCRIPTOR_SET);
            assert_eq!(message.publish_time, 1234);
            fault_protocol::decode_aggregation(&message.data, Default::default()).unwrap();
            count += 1;
        }
    }
    assert_eq!(count, 2);
    std::fs::remove_dir_all(directory).unwrap();
}
