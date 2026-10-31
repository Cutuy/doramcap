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
    writer: RecordingSink,
    channel: u16,
    dictionary_channel: u16,
}
impl McapSink {
    /// Refuses to overwrite an existing recording.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let mut writer = RecordingSink::create(path)?;
        let schema = writer.writer.add_schema(
            telemetry_protocol::MCAP_SCHEMA_NAME,
            "protobuf",
            telemetry_protocol::FILE_DESCRIPTOR_SET,
        )?;
        let metadata =
            BTreeMap::from([("protocol".into(), "doramcap.telemetry.protobuf.v2".into())]);
        let channel =
            writer
                .writer
                .add_channel(schema, "doramcap/snapshots", "protobuf", &metadata)?;
        let dictionary_channel =
            writer
                .writer
                .add_channel(schema, "doramcap/dictionary", "protobuf", &metadata)?;
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
        self.writer.write(
            self.channel,
            // MCAP sequences are 32-bit; payload preserves the full 64-bit sequence.
            sequence as u32,
            capture_unix_ns,
            capture_unix_ns,
            payload,
        )?;
        Ok(())
    }

    pub fn write_dictionary(&mut self, payload: &[u8]) -> Result<()> {
        let frame = telemetry_protocol::decode_dictionary(payload)?;
        self.writer.write(
            self.dictionary_channel,
            frame.revision as u32,
            frame.capture_unix_ns,
            frame.capture_unix_ns,
            payload,
        )?;
        Ok(())
    }

    pub fn finish(self) -> Result<()> {
        self.writer.finish()
    }
}

/// MCAP storage for explicitly declared byte payloads, channels and timestamps.
/// The caller validates payloads and owns source timing and loss accounting.
pub struct RecordingSink {
    writer: mcap::Writer<BufWriter<File>>,
}

impl RecordingSink {
    /// Creates a recording exclusively; parent directories must exist.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::options().write(true).create_new(true).open(path)?;
        Ok(Self {
            writer: mcap::Writer::new(BufWriter::new(file))?,
        })
    }

    /// Adds a channel; `schema: None` declares messages without a schema.
    pub fn add_channel(
        &mut self,
        topic: &str,
        encoding: &str,
        schema: Option<(&str, &str, &[u8])>,
        metadata: &BTreeMap<String, String>,
    ) -> Result<u16> {
        let schema = schema
            .map(|(name, encoding, bytes)| self.writer.add_schema(name, encoding, bytes))
            .transpose()?
            .unwrap_or(0);
        Ok(self.writer.add_channel(schema, topic, encoding, metadata)?)
    }

    /// Writes unchanged bytes with caller-supplied MCAP timing and sequence.
    pub fn write(
        &mut self,
        channel: u16,
        sequence: u32,
        log_time: u64,
        publish_time: u64,
        payload: &[u8],
    ) -> Result<()> {
        self.writer.write_to_known_channel(
            &mcap::records::MessageHeader {
                channel_id: channel,
                sequence,
                log_time,
                publish_time,
            },
            payload,
        )?;
        Ok(())
    }

    /// Stores named recording or transport metadata beside the messages.
    pub fn metadata(&mut self, name: &str, metadata: BTreeMap<String, String>) -> Result<()> {
        self.writer.write_metadata(&mcap::records::Metadata {
            name: name.into(),
            metadata,
        })?;
        Ok(())
    }

    /// Writes the footer and flushes userspace buffers; errors leave partial evidence.
    pub fn finish(mut self) -> Result<()> {
        self.writer.finish()?;
        self.writer.into_inner().flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_channels_keep_schemas_payloads_and_distinct_times() -> Result<()> {
        // Arrange
        let path = std::env::temp_dir().join(format!(
            "configured-mcap-{}-{}.mcap",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let mut sink = RecordingSink::create(&path)?;
        let schema_a = br#"{"type":"object"}"#;
        let schema_b = br#"{"type":"string"}"#;
        let first = sink.add_channel(
            "/sensors",
            "json",
            Some(("Sensors", "jsonschema", schema_a)),
            &BTreeMap::new(),
        )?;
        let second = sink.add_channel(
            "/status",
            "json",
            Some(("Status", "jsonschema", schema_b)),
            &BTreeMap::new(),
        )?;

        // Act
        sink.write(first, 7, 20, 10, b"{}")?;
        sink.write(second, 0, 30, 0, br#""ready""#)?;
        sink.finish()?;

        // Assert
        assert!(RecordingSink::create(&path).is_err());
        let bytes = std::fs::read(&path)?;
        let messages =
            mcap::MessageStream::new(&bytes)?.collect::<std::result::Result<Vec<_>, _>>()?;
        assert_eq!(messages.len(), 2);
        for (message, topic, schema, payload) in [
            (
                &messages[0],
                "/sensors",
                schema_a.as_slice(),
                b"{}".as_slice(),
            ),
            (
                &messages[1],
                "/status",
                schema_b.as_slice(),
                br#""ready""#.as_slice(),
            ),
        ] {
            assert_eq!(message.channel.topic, topic);
            assert_eq!(message.channel.message_encoding, "json");
            assert_eq!(
                message.channel.schema.as_ref().unwrap().data.as_ref(),
                schema
            );
            assert_eq!(message.data.as_ref(), payload);
        }
        assert_eq!(
            (
                messages[0].sequence,
                messages[0].log_time,
                messages[0].publish_time
            ),
            (7, 20, 10)
        );
        assert_eq!(
            (
                messages[1].sequence,
                messages[1].log_time,
                messages[1].publish_time
            ),
            (0, 30, 0)
        );
        std::fs::remove_file(path)?;
        Ok(())
    }

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
