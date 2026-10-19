use monitor_wire::{DictionaryEntry, Frame, Ingress, encode};
fn main() -> eyre::Result<()> {
    let mut snapshot = monitor_core::Snapshot::with_capacity(1);
    snapshot.sequence = u64::MAX;
    snapshot.capture_unix_ns = 1_800_000_000_000_000_001;
    snapshot.monotonic_end_ns = 100_000_000;
    snapshot.push(monitor_core::CpuRecord {
        cgroup_id: u64::MAX,
        cpu: 19,
        flags: 1,
        runtime_ns: u64::MAX,
        ..Default::default()
    });
    snapshot.task_records.push(task_accounting::TaskRecord {
        identity: task_accounting::TaskIdentity {
            start_boottime_ns: u64::MAX,
            cgroup_id: u64::MAX,
            tid: 123,
            tgid: 120,
            cpu: 19,
            flags: 1,
        },
        charged_runtime_ns: u64::MAX,
    });
    let frame = Frame {
        version: 1,
        snapshot,
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
    let mut bytes = Vec::new();
    encode(&frame, &mut bytes)?;
    std::fs::write(std::env::args().nth(1).expect("output path"), bytes)?;
    Ok(())
}
