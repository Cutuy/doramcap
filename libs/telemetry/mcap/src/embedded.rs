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

/// Encoded publication bytes borrowed during [`Recorder::record`]'s callback.
/// Storage commit follows the callback; the borrow cannot be retained afterward.
pub trait Payload {
    /// Encoded byte length.
    fn len(&self) -> usize;
    /// Whether the payload contains zero bytes.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Copy all encoded bytes into a destination of exactly [`Self::len`] bytes.
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
/// Owner-thread telemetry recorder with explicit polling, rotation and finalization.
///
/// Methods perform encoding and I/O synchronously on the calling thread. Mutable
/// access serializes operations. Successful records have been accepted by storage;
/// [`Self::finish`] establishes finalized, synced output. Call it explicitly before
/// treating the recording as complete; dropping the recorder does not seal MCAP.
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
    /// Create a new recording with snapshot and dictionary channels.
    ///
    /// `path` must not exist. `direct` selects O_DIRECT with 4 KiB alignment;
    /// otherwise writes use buffered I/O. `rotation_bytes` is a soft byte target,
    /// with zero disabling the rotation recommendation. Rotation is caller-driven.
    ///
    /// Returns file creation, allocation, registration or io_uring errors. A newly
    /// created partial file may remain on failure; existing files are preserved.
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
        let writer = McapWriter::new(
            file,
            schema,
            &channels,
            storage_persistence::mcap::McapConfig {
                buffers: 8,
                buffer_size: 256 * 1024,
                alignment: if direct { 4096 } else { 1 },
                ..Default::default()
            },
        )?;
        Ok(Self {
            writer,
            base: path.to_path_buf(),
            index: 0,
            rotation_bytes,
            direct,
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
    /// Finalize and sync the current file, then begin the next numbered file.
    ///
    /// Replaces the base path's extension with `000001.mcap`, `000002.mcap`, etc.
    /// Schemas/channels repeat automatically; the caller supplies any dictionary
    /// checkpoint. Successful rotation resets the accepted-snapshot count.
    ///
    /// Returns index exhaustion, file creation, finalization or new-file I/O errors.
    /// An attempted rotation consumes its index, including on failure. Failure may
    /// leave a finalized old file or a partial new file; rotation has no rollback.
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
    /// Refresh completions and flush aged buffers on the caller's thread.
    /// Returns storage errors, including failures discovered after record acceptance.
    /// Call periodically when idle output should advance before the next record.
    pub fn poll(&mut self) -> Result<()> {
        self.writer.poll(Instant::now())?;
        Ok(())
    }
    /// Encode, publish and accept one packet for recording, in that order.
    ///
    /// `dictionary` selects the dictionary channel; otherwise use the snapshot
    /// channel. The caller supplies a matching packet and transport metadata.
    /// `sequence` is narrowed to MCAP's 32-bit header; `time` is Unix nanoseconds
    /// used for both log and publication timestamps. Packet bytes are preserved.
    ///
    /// The synchronous `publish` callback borrows the exact encoded payload until
    /// it returns. Encoding or callback failure abandons the reservation. After a
    /// successful callback, storage commit may still fail: publication and recording
    /// are separate effects, so retries must account for an already published packet.
    ///
    /// Returns frame-size, encoding, callback or storage errors. Success establishes
    /// storage acceptance; later I/O failures surface through record/poll/finish.
    /// Use [`Self::finish`] for finalized, synced output.
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
        let mut reservation = self.writer.reserve(meta, len)?;
        packet.encode(&mut reservation.payload_mut())?;
        publish(&Slice(reservation.payload()))?;
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
                payload.copy_to(&mut published);
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
        }
        std::fs::remove_file(path)?;
        Ok(())
    }
}
