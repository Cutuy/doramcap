//! Publication and recording belong outside the periodic collection thread.
use eyre::Result;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

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
}
