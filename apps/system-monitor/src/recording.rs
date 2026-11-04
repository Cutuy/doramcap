//! Application-level telemetry policy over the format-independent storage writer.
use crate::config::Recording;
use eyre::{Result, ensure};
use prost::Message as _;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
use storage_persistence::mcap::{Channel, McapWriter, Message, Schema};

/// CPU packet recording on the caller thread, with explicit polling and rotation.
/// Finalization drains storage and syncs the output; dropping leaves it unsealed.
pub struct Recorder {
    writer: McapWriter,
    base: PathBuf,
    index: u64,
    rotation_bytes: u64,
    snapshots: u64,
}
impl Recorder {
    /// Create a new recording from a validated section; an existing path is rejected.
    /// `rotation_bytes` is a soft byte target; zero disables rotation recommendations.
    pub fn create(config: &Recording) -> Result<Self> {
        let path = Path::new(&config.path);
        let rotation_bytes = config.rotation_bytes;
        let schema = Schema {
            name: telemetry_protocol::MCAP_SCHEMA_NAME,
            encoding: "protobuf",
            data: telemetry_protocol::FILE_DESCRIPTOR_SET,
        };
        let channels = [
            Channel {
                id: 1,
                topic: &config.samples_topic,
                encoding: "protobuf",
            },
            Channel {
                id: 2,
                topic: &config.dictionary_topic,
                encoding: "protobuf",
            },
        ];
        let writer = McapWriter::create(
            path,
            schema,
            &channels,
            storage_persistence::mcap::McapConfig {
                buffers: 8,
                buffer_size: 256 * 1024,
                ..Default::default()
            },
        )?;
        Ok(Self {
            writer,
            base: path.to_path_buf(),
            index: 0,
            rotation_bytes,
            snapshots: 0,
        })
    }
    /// Whether the soft target was reached after at least one accepted snapshot.
    /// Counts buffered and accepted bytes; a complete record may exceed the target.
    /// Dictionary-only recordings do not trigger this recommendation.
    pub fn rotation_due(&self) -> bool {
        self.snapshots > 0
            && self.rotation_bytes > 0
            && self.writer.file_bytes() >= self.rotation_bytes
    }
    /// Seal this file and open the next numbered file with its schema and channels.
    /// The caller supplies a dictionary checkpoint; failure consumes the index.
    pub fn rotate(&mut self) -> Result<()> {
        self.index = self
            .index
            .checked_add(1)
            .ok_or_else(|| eyre::eyre!("rotation index overflow"))?;
        let path = self.base.with_extension(format!("{:06}.mcap", self.index));
        self.writer.rotate(&path)?;
        self.snapshots = 0;
        Ok(())
    }
    /// Next deadline for buffered recording data; absent while empty.
    pub fn next_flush_deadline(&self) -> Option<Instant> {
        self.writer.next_flush_deadline()
    }
    /// Refresh completions and flush aged buffers on the caller's thread.
    /// Returns storage errors, including failures discovered after record acceptance.
    /// Call at the buffered-data deadline when no records arrive.
    pub fn poll(&mut self) -> Result<()> {
        let now = Instant::now();
        if self
            .writer
            .next_flush_deadline()
            .is_some_and(|deadline| now >= deadline)
        {
            self.writer.flush_written()?;
        } else {
            self.writer.poll(now)?;
        }
        Ok(())
    }
    /// Encode and publish the packet, then commit the same bytes to storage.
    /// The callback borrows bytes until it returns; callback failure abandons them.
    /// Later storage failure can follow successful publication; finish seals output.
    pub fn record(
        &mut self,
        dictionary: bool,
        sequence: u64,
        time: u64,
        packet: &telemetry_protocol::pb::Packet,
        publish: impl FnOnce(&[u8]) -> Result<()>,
    ) -> Result<()> {
        let len = packet.encoded_len();
        ensure!(
            len <= telemetry_protocol::MAX_FRAME_BYTES,
            "frame exceeds protocol limit"
        );
        let meta = Message {
            channel: if dictionary { 2 } else { 1 },
            sequence: sequence as u32,
            log_time: time,
            publish_time: time,
        };
        let mut reservation = self.writer.reserve(meta, len)?;
        packet.encode(&mut reservation.payload_mut())?;
        publish(reservation.payload())?;
        reservation.commit()?;
        if !dictionary {
            self.snapshots += 1;
        }
        Ok(())
    }
    /// Consume the recorder, drain writes, seal MCAP and sync the file.
    /// Returns pending write, footer, truncation or sync failures. On error the
    /// recorder is consumed and the file may be incomplete; inspect the error.
    pub fn finish(self) -> Result<()> {
        self.writer.finish()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_uses_identical_small_and_contiguous_payloads() -> Result<()> {
        // Arrange
        let path =
            std::env::temp_dir().join(format!("doramcap-callback-{}.mcap", std::process::id()));
        let mut recorder = Recorder::create(&Recording {
            path: path.to_str().unwrap().into(),
            samples_topic: "example/cpu".into(),
            dictionary_topic: "example/names".into(),
            ..Default::default()
        })?;
        let mut expected = Vec::new();
        for size in [4, 70000] {
            let packet =
                telemetry_protocol::dictionary_packet(&telemetry_protocol::DictionaryFrame {
                    version: telemetry_protocol::VERSION,
                    session_id: "test".into(),
                    revision: 1,
                    overflow: 0,
                    capture_unix_ns: 12,
                    entries: vec![telemetry_protocol::DictionaryEntry {
                        id: 1,
                        parent_id: 0,
                        path: Some("a".repeat(size)),
                    }],
                });
            // Act / Assert: callback failure abandons this packet.
            assert!(
                recorder
                    .record(true, 1, 12, &packet, |_| eyre::bail!(
                        "injected publication failure"
                    ))
                    .is_err()
            );
            let mut published = vec![0; packet.encoded_len()];
            // Act: retry with a successful publisher.
            recorder.record(true, 1, 12, &packet, |payload| {
                published.copy_from_slice(payload);
                Ok(())
            })?;
            // Assert
            assert_eq!(published, packet.encode_to_vec());
            expected.push(published);
        }
        // Act
        recorder.finish()?;
        let data = std::fs::read(&path)?;
        let messages = mcap::MessageStream::new(&data)?.collect::<Result<Vec<_>, _>>()?;
        // Assert: only the successful publications were recorded.
        assert_eq!(messages.len(), 2);
        for (message, bytes) in messages.iter().zip(expected) {
            assert_eq!(message.data.as_ref(), bytes);
            assert_eq!(message.channel.topic, "example/names");
        }
        std::fs::remove_file(path)?;
        Ok(())
    }
}
