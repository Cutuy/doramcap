//! Protobuf telemetry shared by Dora, binary WebSocket and MCAP.
use prost::Message;
use std::borrow::Borrow;
pub const VERSION: u32 = 2;
pub const MAX_FRAME_BYTES: usize = 128 * 1024 * 1024;
pub const MCAP_SCHEMA_NAME: &str = "doramcap.telemetry.v1.Packet";
pub const FILE_DESCRIPTOR_SET: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/telemetry_descriptor.bin"));
// Keep generated protobuf messages inline to avoid per-packet heap allocation.
#[allow(clippy::large_enum_variant)]
pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.telemetry.v1.rs"));
}
pub use pb::DictionaryEntry;
#[derive(Debug)]
pub struct Frame<S = monitor_core::Snapshot, D = Vec<DictionaryEntry>> {
    pub version: u32,
    pub snapshot: S,
    pub source: String,
    pub task_collection_enabled: bool,
    pub task_capacity: usize,
    pub session_id: String,
    pub dictionary_revision: u64,
    pub dictionary_full: bool,
    pub dictionary_overflow: u64,
    pub dictionary: D,
}
#[derive(Debug)]
pub struct DictionaryFrame<D = Vec<DictionaryEntry>> {
    pub version: u32,
    pub session_id: String,
    pub revision: u64,
    pub overflow: u64,
    pub entries: D,
    pub capture_unix_ns: u64,
}
impl From<&monitor_core::CpuRecord> for pb::CpuRecord {
    fn from(v: &monitor_core::CpuRecord) -> Self {
        Self {
            cgroup_id: v.cgroup_id,
            parent_id: v.parent_id,
            cpu: v.cpu,
            flags: v.flags,
            runtime_ns: v.runtime_ns,
            user_ns: v.user_ns,
            system_ns: v.system_ns,
        }
    }
}
impl From<pb::CpuRecord> for monitor_core::CpuRecord {
    fn from(v: pb::CpuRecord) -> Self {
        Self {
            cgroup_id: v.cgroup_id,
            parent_id: v.parent_id,
            cpu: v.cpu,
            flags: v.flags,
            runtime_ns: v.runtime_ns,
            user_ns: v.user_ns,
            system_ns: v.system_ns,
        }
    }
}
impl From<&task_accounting::TaskIdentity> for pb::TaskIdentity {
    fn from(v: &task_accounting::TaskIdentity) -> Self {
        Self {
            start_boottime_ns: v.start_boottime_ns,
            cgroup_id: v.cgroup_id,
            tid: v.tid,
            tgid: v.tgid,
            cpu: v.cpu,
            flags: v.flags,
        }
    }
}
impl From<pb::TaskIdentity> for task_accounting::TaskIdentity {
    fn from(v: pb::TaskIdentity) -> Self {
        Self {
            start_boottime_ns: v.start_boottime_ns,
            cgroup_id: v.cgroup_id,
            tid: v.tid,
            tgid: v.tgid,
            cpu: v.cpu,
            flags: v.flags,
        }
    }
}
impl From<&task_accounting::TaskRecord> for pb::TaskRecord {
    fn from(v: &task_accounting::TaskRecord) -> Self {
        Self {
            identity: Some((&v.identity).into()),
            charged_runtime_ns: v.charged_runtime_ns,
        }
    }
}
impl From<pb::TaskRecord> for task_accounting::TaskRecord {
    fn from(v: pb::TaskRecord) -> Self {
        Self {
            identity: v.identity.unwrap_or_default().into(),
            charged_runtime_ns: v.charged_runtime_ns,
        }
    }
}
impl From<&task_accounting::Losses> for pb::TaskLosses {
    fn from(v: &task_accounting::Losses) -> Self {
        Self {
            dropped_events: v.dropped_events,
            dropped_runtime_ns: v.dropped_runtime_ns,
            identity_errors: v.identity_errors,
        }
    }
}
impl From<pb::TaskLosses> for task_accounting::Losses {
    fn from(v: pb::TaskLosses) -> Self {
        Self {
            dropped_events: v.dropped_events,
            dropped_runtime_ns: v.dropped_runtime_ns,
            identity_errors: v.identity_errors,
        }
    }
}
impl From<&monitor_core::Snapshot> for pb::Snapshot {
    fn from(v: &monitor_core::Snapshot) -> Self {
        Self {
            schema_version: v.schema_version,
            sequence: v.sequence,
            capture_unix_ns: v.capture_unix_ns,
            monotonic_start_ns: v.monotonic_start_ns,
            monotonic_end_ns: v.monotonic_end_ns,
            dropped_batches: v.dropped_batches,
            capacity_overflow: v.capacity_overflow,
            records: v.records.iter().map(Into::into).collect(),
            task_records: v.task_records.iter().map(Into::into).collect(),
            task_losses: Some((&v.task_losses).into()),
        }
    }
}
impl From<pb::Snapshot> for monitor_core::Snapshot {
    fn from(v: pb::Snapshot) -> Self {
        Self {
            schema_version: v.schema_version,
            sequence: v.sequence,
            capture_unix_ns: v.capture_unix_ns,
            monotonic_start_ns: v.monotonic_start_ns,
            monotonic_end_ns: v.monotonic_end_ns,
            dropped_batches: v.dropped_batches,
            capacity_overflow: v.capacity_overflow,
            records: v.records.into_iter().map(Into::into).collect(),
            task_records: v.task_records.into_iter().map(Into::into).collect(),
            task_losses: v.task_losses.unwrap_or_default().into(),
        }
    }
}

fn encode_packet(packet: pb::Packet, output: &mut Vec<u8>) -> eyre::Result<()> {
    eyre::ensure!(
        packet.encoded_len() <= MAX_FRAME_BYTES,
        "frame exceeds size limit"
    );
    output.clear();
    packet.encode(output)?;
    Ok(())
}
pub fn decode_packet(bytes: &[u8]) -> eyre::Result<pb::Packet> {
    eyre::ensure!(bytes.len() <= MAX_FRAME_BYTES, "frame exceeds size limit");
    let packet = pb::Packet::decode(bytes)?;
    eyre::ensure!(packet.body.is_some(), "missing packet body");
    Ok(packet)
}
pub fn snapshot_packet<S: Borrow<monitor_core::Snapshot>, D: AsRef<[DictionaryEntry]>>(
    frame: &Frame<S, D>,
) -> pb::Packet {
    let value = pb::SnapshotFrame {
        version: frame.version,
        snapshot: Some(frame.snapshot.borrow().into()),
        source: frame.source.clone(),
        task_collection_enabled: frame.task_collection_enabled,
        task_capacity: frame.task_capacity as u64,
        session_id: frame.session_id.clone(),
        dictionary_revision: frame.dictionary_revision,
        dictionary_full: frame.dictionary_full,
        dictionary_overflow: frame.dictionary_overflow,
        dictionary: frame.dictionary.as_ref().to_vec(),
    };
    pb::Packet {
        body: Some(pb::packet::Body::Snapshot(value)),
    }
}
pub fn encode<S: Borrow<monitor_core::Snapshot>, D: AsRef<[DictionaryEntry]>>(
    frame: &Frame<S, D>,
    output: &mut Vec<u8>,
) -> eyre::Result<()> {
    encode_packet(snapshot_packet(frame), output)
}
pub fn decode(bytes: &[u8]) -> eyre::Result<Frame> {
    let Some(pb::packet::Body::Snapshot(f)) = decode_packet(bytes)?.body else {
        eyre::bail!("expected snapshot packet");
    };
    eyre::ensure!(f.version == VERSION, "unsupported snapshot wire version");
    let snapshot = f.snapshot.ok_or_else(|| eyre::eyre!("missing snapshot"))?;
    eyre::ensure!(
        snapshot.schema_version == monitor_core::SCHEMA_VERSION,
        "unsupported snapshot schema"
    );
    eyre::ensure!(
        snapshot.task_records.iter().all(|r| r.identity.is_some()),
        "missing task identity"
    );
    Ok(Frame {
        version: f.version,
        snapshot: snapshot.into(),
        source: f.source,
        task_collection_enabled: f.task_collection_enabled,
        task_capacity: f.task_capacity.try_into()?,
        session_id: f.session_id,
        dictionary_revision: f.dictionary_revision,
        dictionary_full: f.dictionary_full,
        dictionary_overflow: f.dictionary_overflow,
        dictionary: f.dictionary,
    })
}
pub fn dictionary_packet<D: AsRef<[DictionaryEntry]>>(f: &DictionaryFrame<D>) -> pb::Packet {
    pb::Packet {
        body: Some(pb::packet::Body::Dictionary(pb::DictionaryFrame {
            version: f.version,
            session_id: f.session_id.clone(),
            revision: f.revision,
            overflow: f.overflow,
            entries: f.entries.as_ref().to_vec(),
            capture_unix_ns: f.capture_unix_ns,
        })),
    }
}
pub fn encode_dictionary<D: AsRef<[DictionaryEntry]>>(
    f: &DictionaryFrame<D>,
    output: &mut Vec<u8>,
) -> eyre::Result<()> {
    encode_packet(dictionary_packet(f), output)
}
pub fn decode_dictionary(bytes: &[u8]) -> eyre::Result<DictionaryFrame> {
    let Some(pb::packet::Body::Dictionary(f)) = decode_packet(bytes)?.body else {
        eyre::bail!("expected dictionary packet");
    };
    eyre::ensure!(f.version == VERSION, "unsupported dictionary wire version");
    Ok(DictionaryFrame {
        version: f.version,
        session_id: f.session_id,
        revision: f.revision,
        overflow: f.overflow,
        entries: f.entries,
        capture_unix_ns: f.capture_unix_ns,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_matches_checked_in_validation_schema() {
        assert_eq!(
            FILE_DESCRIPTOR_SET,
            include_bytes!("../proto/telemetry_descriptor.bin")
        );
        let descriptor = prost_types::FileDescriptorSet::decode(FILE_DESCRIPTOR_SET).unwrap();
        assert_eq!(
            descriptor.file[0].package.as_deref(),
            Some("doramcap.telemetry.v1")
        );
        assert!(
            descriptor.file[0]
                .message_type
                .iter()
                .any(|m| m.name.as_deref() == Some("Packet"))
        );
    }
    #[test]
    fn binary_roundtrip_and_rejects_corruption() {
        let mut snapshot = monitor_core::Snapshot::with_capacity(1);
        snapshot.schema_version = 1;
        snapshot.sequence = u64::MAX;
        snapshot.capture_unix_ns = 1_800_000_000_000_000_001;
        snapshot.push(monitor_core::CpuRecord {
            cgroup_id: u64::MAX,
            runtime_ns: u64::MAX,
            ..Default::default()
        });
        let mut frame = Frame {
            version: VERSION,
            snapshot: &snapshot,
            source: "bpf".into(),
            task_collection_enabled: false,
            task_capacity: 0,
            session_id: "test".into(),
            dictionary_revision: 0,
            dictionary_full: true,
            dictionary_overflow: 0,
            dictionary: Vec::<DictionaryEntry>::new(),
        };
        let mut bytes = Vec::new();
        encode(&frame, &mut bytes).unwrap();
        assert_eq!(decode(&bytes).unwrap().snapshot.sequence, u64::MAX);
        assert!(!bytes.windows(10).any(|w| w == b"runtime_ns"));
        assert!(decode(&bytes[..bytes.len() - 1]).is_err());
        // Unknown additive protobuf field 100 is compatible.
        bytes.extend_from_slice(&[0xa0, 0x06, 0x01]);
        assert_eq!(decode(&bytes).unwrap().snapshot.sequence, u64::MAX);
        bytes.push(0);
        assert!(decode(&bytes).is_err());
        frame.version = 99;
        encode(&frame, &mut bytes).unwrap();
        assert!(decode(&bytes).is_err());
    }
}

#[cfg(test)]
mod dictionary_tests {
    use super::*;
    #[test]
    fn full_dictionary_roundtrip_and_validation() {
        let frame = DictionaryFrame {
            version: VERSION,
            session_id: "s".into(),
            revision: u64::MAX,
            overflow: 0,
            capture_unix_ns: 10,
            entries: vec![DictionaryEntry {
                id: u64::MAX,
                parent_id: 0,
                path: Some("/test".into()),
            }],
        };
        let mut bytes = Vec::new();
        encode_dictionary(&frame, &mut bytes).unwrap();
        assert_eq!(decode_dictionary(&bytes).unwrap().revision, u64::MAX);
        assert!(decode_dictionary(&bytes[..bytes.len() - 1]).is_err());
        bytes.push(0);
        assert!(decode_dictionary(&bytes).is_err());
    }
}
