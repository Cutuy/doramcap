//! Publication and recording belong outside the periodic collection thread.
use eyre::{Result, ensure};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

/// Separate downstream MCAP writer. This path allocates and performs disk I/O.
pub struct McapSink {
    writer: mcap::Writer<BufWriter<File>>,
    channel: u16,
    dictionary_channel: u16,
}
impl McapSink {
    /// Refuses to overwrite an existing recording.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::options().write(true).create_new(true).open(path)?;
        let mut writer = mcap::Writer::new(BufWriter::new(file))?;
        let schema = writer.add_schema(
            telemetry_protocol::MCAP_SCHEMA_NAME,
            "protobuf",
            telemetry_protocol::FILE_DESCRIPTOR_SET,
        )?;
        let metadata =
            BTreeMap::from([("protocol".into(), "doramcap.telemetry.protobuf.v2".into())]);
        let channel = writer.add_channel(schema, "doramcap/snapshots", "protobuf", &metadata)?;
        let dictionary_channel =
            writer.add_channel(schema, "doramcap/dictionary", "protobuf", &metadata)?;
        Ok(Self {
            writer,
            channel,
            dictionary_channel,
        })
    }

    pub fn write(&mut self, sequence: u64, capture_unix_ns: u64, payload: &[u8]) -> Result<()> {
        // Validate the envelope at the recording boundary, never on collection.
        let value = telemetry_protocol::decode(payload)?;
        ensure!(
            value.snapshot.sequence == sequence,
            "payload sequence differs from transport metadata"
        );
        ensure!(
            value.snapshot.capture_unix_ns == capture_unix_ns,
            "payload capture time differs from transport metadata"
        );
        self.writer.write_to_known_channel(
            &mcap::records::MessageHeader {
                channel_id: self.channel,
                // MCAP sequences are 32-bit; payload preserves the full 64-bit sequence.
                sequence: sequence as u32,
                log_time: capture_unix_ns,
                publish_time: capture_unix_ns,
            },
            payload,
        )?;
        Ok(())
    }

    pub fn write_dictionary(&mut self, payload: &[u8]) -> Result<()> {
        let frame = telemetry_protocol::decode_dictionary(payload)?;
        self.writer.write_to_known_channel(
            &mcap::records::MessageHeader {
                channel_id: self.dictionary_channel,
                sequence: frame.revision as u32,
                log_time: frame.capture_unix_ns,
                publish_time: frame.capture_unix_ns,
            },
            payload,
        )?;
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        self.writer.finish()?;
        self.writer.into_inner().flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(sequence: u64, capture: u64) -> Result<Vec<u8>> {
        let mut snapshot = monitor_core::Snapshot::with_capacity(1);
        snapshot.sequence = sequence;
        snapshot.capture_unix_ns = capture;
        let frame = telemetry_protocol::Frame {
            version: telemetry_protocol::VERSION,
            snapshot,
            source: "test".into(),
            task_collection_enabled: false,
            task_capacity: 0,
            session_id: "test".into(),
            dictionary_revision: 0,
            dictionary_full: true,
            dictionary_overflow: 0,
            dictionary: Vec::<telemetry_protocol::DictionaryEntry>::new(),
            viewer_ingress: None,
        };
        let mut bytes = Vec::new();
        telemetry_protocol::encode(&frame, &mut bytes)?;
        Ok(bytes)
    }
    #[test]
    fn round_trip_preserves_payload_time_and_sequence() -> Result<()> {
        let path = std::env::temp_dir().join(format!(
            "doramcap-output-{}-{}.mcap",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let mut sink = McapSink::create(&path)?;
        assert!(McapSink::create(&path).is_err());
        for sequence in 0..10_u64 {
            let time = 1_800_000_000_000_000_000 + sequence * 100_000_000;
            let payload = sample(sequence, time)?;
            sink.write(sequence, time, &payload)?;
        }
        assert!(sink.write(10, 12, b"invalid").is_err());
        assert!(sink.write(10, 12, &sample(11, 12)?).is_err());
        sink.finish()?;
        let bytes = std::fs::read(&path)?;
        let messages =
            mcap::MessageStream::new(&bytes)?.collect::<std::result::Result<Vec<_>, _>>()?;
        assert_eq!(messages.len(), 10);
        for (i, message) in messages.iter().enumerate() {
            assert_eq!(message.sequence, i as u32);
            assert_eq!(
                message.log_time,
                1_800_000_000_000_000_000 + i as u64 * 100_000_000
            );
            assert_eq!(message.publish_time, message.log_time);
            assert_eq!(message.channel.message_encoding, "protobuf");
            let schema = message
                .channel
                .schema
                .as_ref()
                .expect("embedded Protobuf schema");
            assert_eq!(schema.name, telemetry_protocol::MCAP_SCHEMA_NAME);
            assert_eq!(schema.encoding, "protobuf");
            assert_eq!(
                schema.data.as_ref(),
                telemetry_protocol::FILE_DESCRIPTOR_SET
            );
            let value = telemetry_protocol::decode(&message.data)?;
            assert_eq!(value.snapshot.capture_unix_ns, message.log_time);
        }
        std::fs::remove_file(path)?;
        Ok(())
    }
}

pub mod embedded;
