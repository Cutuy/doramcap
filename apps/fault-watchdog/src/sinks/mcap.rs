//! MCAP domain recording through the repository's storage writer.
use super::{Packet, Sink};
use crate::config::pb::McapConfig;
use eyre::{Result, ensure};
use std::{
    fs::OpenOptions,
    time::{Duration, Instant},
};
use storage_persistence::mcap::{Channel, McapWriter, Message, Schema};
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
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&config.path)?;
        let writer = McapWriter::new(
            file,
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
                buffer_size: max_frame + 4096,
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
            writer.rotate(OpenOptions::new().write(true).create_new(true).open(path)?)?;
        }
        let frame = fault_protocol::decode_aggregation(&packet.bytes, Default::default())?;
        let now = crate::unix_ns().unwrap_or(0);
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
