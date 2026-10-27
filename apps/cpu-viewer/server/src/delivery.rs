//! Viewer-owned delivery annotations, with the legacy browser wire layout.
use prost::Message;

// Keep generated messages inline, matching the producer codec's allocation policy.
#[allow(clippy::large_enum_variant)]
mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.viewer.v1.rs"));
}
pub use pb::{Ingress, Packet, SnapshotFrame, packet};

/// Encode a validated producer frame with observations from this viewer process.
/// The caller owns the frame; no producer state is changed. Original measurement
/// fields and the optional metric presence are preserved. Returns an error when
/// the annotated packet exceeds the shared frame limit; no bytes are published.
pub fn encode_snapshot(
    frame: telemetry_protocol::Frame,
    ingress: Ingress,
) -> eyre::Result<Vec<u8>> {
    let telemetry_protocol::pb::packet::Body::Snapshot(frame) =
        telemetry_protocol::snapshot_packet(&frame)
            .body
            .expect("snapshot packet")
    else {
        unreachable!("snapshot encoder returned dictionary")
    };
    // Exhaustive transfer makes additions to the producer envelope explicit.
    let telemetry_protocol::pb::SnapshotFrame {
        version,
        snapshot,
        source,
        task_collection_enabled,
        task_capacity,
        session_id,
        dictionary_revision,
        dictionary_full,
        dictionary_overflow,
        dictionary,
    } = frame;
    let packet = Packet {
        body: Some(packet::Body::Snapshot(SnapshotFrame {
            version,
            snapshot,
            source,
            task_collection_enabled,
            task_capacity,
            session_id,
            dictionary_revision,
            dictionary_full,
            dictionary_overflow,
            dictionary,
            viewer_ingress: Some(ingress),
        })),
    };
    eyre::ensure!(
        packet.encoded_len() <= telemetry_protocol::MAX_FRAME_BYTES,
        "frame exceeds size limit"
    );
    Ok(packet.encode_to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    const LEGACY: &[u8] = include_bytes!("../../frontend/src/fixtures/snapshot-v2.pb");

    #[test]
    fn viewer_delivery_keeps_legacy_browser_bytes_and_metric_presence() {
        // Arrange
        let producer = telemetry_protocol::decode(LEGACY).unwrap();
        let legacy = Packet::decode(LEGACY).unwrap();
        let Some(packet::Body::Snapshot(legacy)) = legacy.body else {
            panic!("expected snapshot fixture");
        };
        let metrics = legacy.viewer_ingress.unwrap();
        assert_eq!(metrics.total_bytes, u64::MAX);
        assert_eq!(metrics.bytes_per_second, Some(1234.5));
        assert_eq!(metrics.capture_to_receive_ms, None);
        // Act
        let delivered = encode_snapshot(producer, metrics).unwrap();
        // Assert
        assert_eq!(delivered, LEGACY);
        let decoded = telemetry_protocol::decode(&delivered).unwrap();
        assert_eq!(decoded.snapshot.sequence, u64::MAX);
        assert_eq!(
            decoded.snapshot.task_records[0].charged_runtime_ns,
            u64::MAX
        );
    }

    #[test]
    fn viewer_accepts_unannotated_producer_and_dictionary_packets() {
        // Arrange
        let producer = telemetry_protocol::decode(LEGACY).unwrap();
        let mut original = Vec::new();
        telemetry_protocol::encode(&producer, &mut original).unwrap();
        // Act
        let delivered = Packet::decode(original.as_slice()).unwrap();
        // Assert
        let Some(packet::Body::Snapshot(frame)) = delivered.body else {
            panic!("expected snapshot");
        };
        assert!(frame.viewer_ingress.is_none());
        assert_eq!(frame.snapshot.unwrap().sequence, u64::MAX);
        // Arrange
        let dictionary = telemetry_protocol::DictionaryFrame {
            version: telemetry_protocol::VERSION,
            session_id: "fixture".into(),
            revision: u64::MAX,
            overflow: 0,
            entries: Vec::<telemetry_protocol::DictionaryEntry>::new(),
            capture_unix_ns: 17,
        };
        let mut original = Vec::new();
        telemetry_protocol::encode_dictionary(&dictionary, &mut original).unwrap();
        // Act
        let delivered = Packet::decode(original.as_slice()).unwrap();
        // Assert
        let Some(packet::Body::Dictionary(frame)) = delivered.body else {
            panic!("expected dictionary");
        };
        assert_eq!(frame.revision, u64::MAX);
        assert_eq!(frame.session_id, "fixture");
    }
}
