//! MCAP domain recording through the repository's storage writer.
use super::{Packet, Sink};
use crate::config::pb::McapConfig;
use eyre::{Result, ensure};
use std::time::{Duration, Instant};
use storage_persistence::mcap::{ALIGNMENT, Channel, McapWriter, Message, Schema};
/// Owns one writer and numbered rotation files. Existing paths are never replaced.
/// A write/flush failure poisons recording: partial writes cannot safely be retried.
pub struct Recording {
    writer: Option<McapWriter>,
    config: McapConfig,
    part: u64,
    sequence: u32,
    last_flush: Instant,
    poisoned: bool,
}
impl Recording {
    /// Create the initial recording and embed the shared aggregation descriptor.
    /// Requires the Linux capabilities supported by `storage-persistence`.
    pub fn new(config: McapConfig, max_frame: usize) -> Result<Self> {
        let writer = McapWriter::create(
            &config.path,
            Schema {
                name: fault_protocol::AGGREGATION_SCHEMA_NAME,
                encoding: "protobuf",
                data: fault_protocol::FILE_DESCRIPTOR_SET,
            },
            &[Channel {
                id: 1,
                topic: "faults",
                encoding: "protobuf",
            }],
            storage_persistence::mcap::McapConfig {
                buffers: 4,
                // One block for the record header and one for padding keep every frame inline and ordered.
                buffer_size: (max_frame + 2 * ALIGNMENT).next_multiple_of(ALIGNMENT),
                ..Default::default()
            },
        )?;
        Ok(Self {
            writer: Some(writer),
            config,
            part: 0,
            sequence: 0,
            last_flush: Instant::now(),
            poisoned: false,
        })
    }
    fn append(&mut self, packet: &Packet) -> Result<()> {
        let writer = self.writer.as_mut().unwrap();
        if self.config.rotation_bytes > 0 && writer.file_bytes() >= self.config.rotation_bytes {
            self.part += 1;
            let path = format!("{}.{:06}", self.config.path, self.part);
            writer.rotate(path)?;
        }
        let frame = fault_protocol::decode_aggregation(&packet.bytes, Default::default())?;
        let now = fault_protocol::host::unix_ns().unwrap_or(0);
        let mut reservation = writer.reserve(
            Message {
                channel: 1,
                sequence: self.sequence,
                log_time: now,
                publish_time: frame.published_unix_ns.unwrap_or(now),
            },
            packet.bytes.len(),
        )?;
        reservation.payload_mut().copy_from_slice(&packet.bytes);
        reservation.commit()?;
        self.sequence = self.sequence.wrapping_add(1);
        Ok(())
    }
}
impl Sink for Recording {
    fn write(&mut self, packet: &Packet) -> Result<()> {
        ensure!(!self.poisoned, "recording disabled after I/O failure");
        let result = self.append(packet);
        self.poisoned = result.is_err();
        result
    }
    fn tick(&mut self) -> Result<()> {
        if self.poisoned {
            return Ok(());
        }
        let writer = self.writer.as_mut().unwrap();
        let result = (|| -> Result<()> {
            writer.poll(Instant::now())?;
            if self.last_flush.elapsed() >= Duration::from_millis(self.config.flush_ms.into()) {
                writer.flush_written()?;
                self.last_flush = Instant::now();
            }
            Ok(())
        })();
        self.poisoned = result.is_err();
        result
    }
    fn finish(mut self: Box<Self>) -> Result<()> {
        let result = self.writer.take().unwrap().finish();
        result?;
        ensure!(!self.poisoned, "recording encountered an I/O failure");
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn largest_frame_fits_combining_buffer() {
        for max_frame in [4090, 2 * 1024 * 1024] {
            // Arrange
            let path = std::env::temp_dir().join(format!(
                "fault-mcap-inline-{}-{max_frame}",
                fault_protocol::host::incarnation().unwrap()
            ));
            let config = McapConfig {
                enabled: true,
                path: path.to_str().unwrap().into(),
                rotation_bytes: 0,
                flush_ms: 1,
            };

            // Act
            let recording = Recording::new(config, max_frame).unwrap();

            // Assert
            let inline = recording.writer.as_ref().unwrap().max_inline_payload();
            assert!(inline >= max_frame, "{inline} < {max_frame}");
            Box::new(recording).finish().unwrap();
            std::fs::remove_file(path).unwrap();
        }
    }
}
