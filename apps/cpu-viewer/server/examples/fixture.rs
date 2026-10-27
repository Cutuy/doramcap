#[path = "../src/delivery.rs"]
#[allow(dead_code)]
mod delivery;
use delivery::{Ingress, Packet, SnapshotFrame, packet};
use prost::Message;
use telemetry_protocol::pb::{CpuRecord, DictionaryEntry, Snapshot, TaskIdentity, TaskRecord};
fn fixture() -> Vec<u8> {
    let mut snapshot = Snapshot {
        schema_version: 1,
        task_losses: Some(Default::default()),
        ..Default::default()
    };
    snapshot.sequence = u64::MAX;
    snapshot.capture_unix_ns = 1_800_000_000_000_000_001;
    snapshot.monotonic_end_ns = 100_000_000;
    snapshot.records.push(CpuRecord {
        cgroup_id: u64::MAX,
        cpu: 19,
        flags: 1,
        runtime_ns: u64::MAX,
        ..Default::default()
    });
    snapshot.task_records.push(TaskRecord {
        identity: Some(TaskIdentity {
            start_boottime_ns: u64::MAX,
            cgroup_id: u64::MAX,
            tid: 123,
            tgid: 120,
            cpu: 19,
            flags: 1,
        }),
        charged_runtime_ns: u64::MAX,
    });
    let frame = SnapshotFrame {
        version: telemetry_protocol::VERSION,
        snapshot: Some(snapshot),
        source: "bpf".into(),
        task_collection_enabled: true,
        task_capacity: 32768,
        session_id: "fixture".into(),
        dictionary_revision: 1,
        dictionary_full: true,
        dictionary_overflow: 0,
        dictionary: vec![DictionaryEntry {
            id: u64::MAX,
            parent_id: 0,
            path: Some("test.scope".into()),
        }],
        viewer_ingress: Some(Ingress {
            total_bytes: u64::MAX,
            total_messages: 1,
            bytes_per_second: Some(1234.5),
            ..Default::default()
        }),
    };
    Packet {
        body: Some(packet::Body::Snapshot(frame)),
    }
    .encode_to_vec()
}

fn main() -> eyre::Result<()> {
    std::fs::write(std::env::args().nth(1).expect("output path"), fixture())?;
    Ok(())
}

#[test]
fn generated_fixture_keeps_legacy_browser_bytes() {
    // Arrange
    let legacy = include_bytes!("../../frontend/src/fixtures/snapshot-v2.pb");
    // Act
    let generated = fixture();
    // Assert
    assert_eq!(generated, legacy);
}
