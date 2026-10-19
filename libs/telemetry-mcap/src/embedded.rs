//! Application-level telemetry policy over the format-independent storage writer.
use eyre::{Result, ensure};
use prost::Message as _;
use std::{
    fs::File,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::Instant,
};
use storage_persistence::mcap::{Channel, LargeReservation, McapWriter, Message, Schema};

/// Borrow lasts through the publication callback; storage commit follows it.
pub trait Payload {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn copy_to(&self, destination: &mut [u8]);
}
struct Slice<'a>(&'a [u8]);
impl Payload for Slice<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn copy_to(&self, destination: &mut [u8]) {
        destination.copy_from_slice(self.0);
    }
}
impl Payload for LargeReservation<'_> {
    fn len(&self) -> usize {
        self.payload_len()
    }
    fn copy_to(&self, destination: &mut [u8]) {
        self.copy_payload_to(destination);
    }
}
pub struct Recorder {
    writer: McapWriter,
    base: PathBuf,
    index: u64,
    rotation_bytes: u64,
    direct: bool,
    snapshots: u64,
}
fn create(path: &Path, direct: bool) -> Result<File> {
    Ok(File::options()
        .write(true)
        .create_new(true)
        .custom_flags(if direct { libc::O_DIRECT } else { 0 })
        .open(path)?)
}
impl Recorder {
    pub fn create(path: &Path, direct: bool, rotation_bytes: u64) -> Result<Self> {
        let file = create(path, direct)?;
        let schema = Schema {
            name: telemetry_protocol::MCAP_SCHEMA_NAME,
            encoding: "protobuf",
            data: telemetry_protocol::FILE_DESCRIPTOR_SET,
        };
        let channels = [
            Channel {
                id: 1,
                topic: "doramcap/snapshots",
                encoding: "protobuf",
            },
            Channel {
                id: 2,
                topic: "doramcap/dictionary",
                encoding: "protobuf",
            },
        ];
        let writer = if direct {
            McapWriter::direct(file, 8, 64 * 1024, schema, &channels, 4096)?
        } else {
            McapWriter::new(file, 8, 64 * 1024, schema, &channels)?
        };
        Ok(Self {
            writer,
            base: path.to_path_buf(),
            index: 0,
            rotation_bytes,
            direct,
            snapshots: 0,
        })
    }
    pub fn rotation_due(&self) -> bool {
        self.snapshots > 0
            && self.rotation_bytes > 0
            && self.writer.file_bytes() >= self.rotation_bytes
    }
    pub fn rotate(&mut self) -> Result<()> {
        self.index = self
            .index
            .checked_add(1)
            .ok_or_else(|| eyre::eyre!("rotation index overflow"))?;
        let path = self.base.with_extension(format!("{:06}.mcap", self.index));
        self.writer.rotate(create(&path, self.direct)?)?;
        self.snapshots = 0;
        Ok(())
    }
    pub fn poll(&mut self) -> Result<()> {
        self.writer.poll(Instant::now())?;
        Ok(())
    }
    pub fn record(
        &mut self,
        dictionary: bool,
        sequence: u64,
        time: u64,
        packet: &telemetry_protocol::pb::Packet,
        publish: impl FnOnce(&dyn Payload) -> Result<()>,
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
        if len <= self.writer.max_inline_payload() {
            let mut reservation = self.writer.reserve(meta, len)?;
            packet.encode(&mut reservation.payload_mut())?;
            publish(&Slice(reservation.payload()))?;
            reservation.commit()?;
        } else {
            let mut reservation = self.writer.reserve_large(meta, len)?;
            packet.encode(&mut reservation)?;
            publish(&reservation)?;
            reservation.commit()?;
        }
        if !dictionary {
            self.snapshots += 1;
        }
        Ok(())
    }
    pub fn finish(self) -> Result<()> {
        self.writer.finish()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_uses_identical_small_and_segmented_payloads() -> Result<()> {
        let path =
            std::env::temp_dir().join(format!("doramcap-callback-{}.mcap", std::process::id()));
        let mut recorder = Recorder::create(&path, true, 0)?;
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
            assert!(
                recorder
                    .record(true, 1, 12, &packet, |_| eyre::bail!(
                        "injected publication failure"
                    ))
                    .is_err()
            );
            let mut published = vec![0; packet.encoded_len()];
            recorder.record(true, 1, 12, &packet, |payload| {
                payload.copy_to(&mut published);
                Ok(())
            })?;
            assert_eq!(published, packet.encode_to_vec());
            expected.push(published);
        }
        recorder.finish()?;
        let data = std::fs::read(&path)?;
        let messages = mcap::MessageStream::new(&data)?.collect::<Result<Vec<_>, _>>()?;
        assert_eq!(messages.len(), 2);
        for (message, bytes) in messages.iter().zip(expected) {
            assert_eq!(message.data.as_ref(), bytes);
        }
        std::fs::remove_file(path)?;
        Ok(())
    }
}
