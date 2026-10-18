//! Publication and recording belong outside the periodic collection thread.
use eyre::{ensure, Result};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

#[cfg(feature = "dora")]
mod publication;
#[cfg(feature = "dora")]
pub use publication::Publisher;

/// JSON envelope intentionally permits evolving backend-specific numeric fields.
/// The full snapshot remains in the payload; these mandatory fields identify it.
pub const SNAPSHOT_SCHEMA: &str = r#"{
  "$schema":"http://json-schema.org/draft-07/schema#",
  "title":"doramcap snapshot v1",
  "type":"object",
  "required":["sequence","capture_unix_ns"],
  "properties":{
    "sequence":{"type":"integer","minimum":0},
    "capture_unix_ns":{"type":"integer","minimum":0}
  },
  "additionalProperties":true
}"#;

/// Separate downstream MCAP writer. This path allocates and performs disk I/O.
pub struct McapSink {
    writer: mcap::Writer<BufWriter<File>>,
    channel: u16,
}
impl McapSink {
    /// Refuses to overwrite an existing recording.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::options().write(true).create_new(true).open(path)?;
        let mut writer = mcap::Writer::new(BufWriter::new(file))?;
        let schema = writer.add_schema(
            "doramcap.snapshot.v1",
            "jsonschema",
            SNAPSHOT_SCHEMA.as_bytes(),
        )?;
        let channel = writer.add_channel(schema, "doramcap/snapshots", "json", &BTreeMap::new())?;
        Ok(Self { writer, channel })
    }

    pub fn write(&mut self, sequence: u64, capture_unix_ns: u64, payload: &[u8]) -> Result<()> {
        // Validate the envelope at the recording boundary, never on collection.
        let value: serde_json::Value = serde_json::from_slice(payload)?;
        ensure!(
            value["sequence"].as_u64() == Some(sequence),
            "payload sequence differs from transport metadata"
        );
        ensure!(
            value["capture_unix_ns"].as_u64() == Some(capture_unix_ns),
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
            let payload =
                format!(r#"{{"sequence":{sequence},"capture_unix_ns":{time},"runtime_ns":123}}"#);
            sink.write(sequence, time, payload.as_bytes())?;
        }
        assert!(sink.write(10, 12, b"not JSON").is_err());
        assert!(sink
            .write(10, 12, br#"{"sequence":11,"capture_unix_ns":12}"#)
            .is_err());
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
            assert_eq!(message.channel.message_encoding, "json");
            assert_eq!(
                message.channel.schema.as_ref().unwrap().encoding,
                "jsonschema"
            );
            let value: serde_json::Value = serde_json::from_slice(&message.data)?;
            assert_eq!(value["runtime_ns"], 123);
            assert_eq!(value["capture_unix_ns"], message.log_time);
        }
        std::fs::remove_file(path)?;
        Ok(())
    }
}
