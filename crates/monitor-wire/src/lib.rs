//! Versioned MessagePack arrays. Field order is the wire ABI; see docs/wire-protocol.md.
use serde::{Deserialize, Serialize};
pub const VERSION: u32 = 1;
pub const MAX_FRAME_BYTES: usize = 128 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub id: u64,
    pub parent_id: u64,
    pub path: Option<String>,
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Ingress {
    pub total_bytes: u64,
    pub total_messages: u64,
    pub bytes_per_second: Option<f64>,
    pub messages_per_second: Option<f64>,
    pub capture_to_receive_ms: Option<f64>,
    pub mean_capture_to_receive_ms: Option<f64>,
    pub invalid_timestamps: u64,
}
#[derive(Debug, Serialize, Deserialize)]
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
    pub viewer_ingress: Option<Ingress>,
}
pub fn encode<S: Serialize, D: Serialize>(
    frame: &Frame<S, D>,
    output: &mut Vec<u8>,
) -> eyre::Result<()> {
    output.clear();
    // Default serializer uses positional arrays, NOT named maps.
    frame.serialize(&mut rmp_serde::Serializer::new(&mut *output))?;
    eyre::ensure!(
        output.len() <= MAX_FRAME_BYTES,
        "snapshot exceeds wire size limit"
    );
    Ok(())
}
pub fn decode(bytes: &[u8]) -> eyre::Result<Frame> {
    eyre::ensure!(
        bytes.len() <= MAX_FRAME_BYTES,
        "snapshot exceeds wire size limit"
    );
    let mut cursor = std::io::Cursor::new(bytes);
    let frame: Frame = Deserialize::deserialize(&mut rmp_serde::Deserializer::new(&mut cursor))?;
    eyre::ensure!(
        cursor.position() == bytes.len() as u64,
        "trailing snapshot bytes"
    );
    eyre::ensure!(
        frame.version == VERSION,
        "unsupported snapshot wire version {}",
        frame.version
    );
    eyre::ensure!(
        frame.snapshot.schema_version == monitor_core::SCHEMA_VERSION,
        "unsupported snapshot schema"
    );
    Ok(frame)
}
/// Full dictionary checkpoint on the low-rate topic. Separate array shape from Frame.
#[derive(Debug, Serialize, Deserialize)]
pub struct DictionaryFrame<D = Vec<DictionaryEntry>> {
    pub version: u32,
    pub session_id: String,
    pub revision: u64,
    pub overflow: u64,
    pub entries: D,
    pub capture_unix_ns: u64,
}
pub fn encode_dictionary<D: Serialize>(
    frame: &DictionaryFrame<D>,
    output: &mut Vec<u8>,
) -> eyre::Result<()> {
    output.clear();
    frame.serialize(&mut rmp_serde::Serializer::new(&mut *output))?;
    eyre::ensure!(
        output.len() <= MAX_FRAME_BYTES,
        "dictionary exceeds size limit"
    );
    Ok(())
}
pub fn decode_dictionary(bytes: &[u8]) -> eyre::Result<DictionaryFrame> {
    eyre::ensure!(
        bytes.len() <= MAX_FRAME_BYTES,
        "dictionary exceeds size limit"
    );
    let mut cursor = std::io::Cursor::new(bytes);
    let frame: DictionaryFrame =
        Deserialize::deserialize(&mut rmp_serde::Deserializer::new(&mut cursor))?;
    eyre::ensure!(
        cursor.position() == bytes.len() as u64,
        "trailing dictionary bytes"
    );
    eyre::ensure!(
        frame.version == VERSION,
        "unsupported dictionary wire version"
    );
    Ok(frame)
}
#[cfg(test)]
mod tests {
    use super::*;
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
            viewer_ingress: None,
        };
        let mut bytes = Vec::new();
        encode(&frame, &mut bytes).unwrap();
        assert_eq!(decode(&bytes).unwrap().snapshot.sequence, u64::MAX);
        assert!(!bytes.windows(10).any(|w| w == b"runtime_ns"));
        assert!(decode(&bytes[..bytes.len() - 1]).is_err());
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
