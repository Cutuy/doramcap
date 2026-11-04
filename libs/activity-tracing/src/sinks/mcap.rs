use crate::{BatchView, Blocks, Result, Sink, SinkBuilder, wire};
use std::{
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use storage_persistence::mcap::{Channel, McapConfig, McapWriter, Message, Schema};

const CHANNEL: u16 = 1;
/// Upper bound added to persistence latency by combining blocks into one message.
const BATCH_INTERVAL: Duration = Duration::from_millis(10);

/// Direct-I/O MCAP output; its writer is created on the sink worker that owns it.
pub struct McapSink {
    path: PathBuf,
}
impl McapSink {
    /// Names a new recording. [`crate::Runtime::start`] creates it and fails on an existing path.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }
}
impl SinkBuilder for McapSink {
    fn build(self: Box<Self>) -> Result<Box<dyn Sink>> {
        let writer = McapWriter::create(
            self.path,
            Schema {
                name: wire::SCHEMA,
                encoding: "protobuf",
                data: wire::DESCRIPTOR,
            },
            &[Channel {
                id: CHANNEL,
                topic: "doramcap/activity",
                encoding: "protobuf",
            }],
            McapConfig {
                buffers: 8,
                buffer_size: 256 * 1024,
                ..Default::default()
            },
        )?;
        Ok(Box::new(Writer(Some(writer))))
    }
}

struct Writer(Option<McapWriter>);
impl Sink for Writer {
    fn name(&self) -> &str {
        "mcap"
    }
    fn write(&mut self, _: BatchView<'_>) -> Result<()> {
        Err("McapSink serializes in write_blocks".into())
    }
    /// Serializes each batch directly into the writer's pinned buffer.
    fn write_blocks(&mut self, blocks: &mut Blocks<'_>) -> Result<()> {
        let writer = self.0.as_mut().ok_or("MCAP finalized")?;
        let limit = writer.max_inline_payload();
        while let Some(plan) = blocks.plan(limit) {
            let message = Message {
                channel: CHANNEL,
                sequence: plan.sequence as u32,
                log_time: plan.unix_ns,
                publish_time: plan.unix_ns,
            };
            // Drain first so commit has pending room and large allocations stay one at a time.
            if plan.len > limit {
                writer.flush_written()?;
            }
            let mut flushed = false;
            loop {
                match writer.reserve(message, plan.len) {
                    Ok(mut reservation) => {
                        blocks.encode(&plan, reservation.payload_mut());
                        reservation.try_commit()?;
                        break;
                    }
                    // Flushing frees everything except registrations, so a second refusal is permanent.
                    Err(error) if flushed || error.kind() != io::ErrorKind::WouldBlock => {
                        return Err(error.into());
                    }
                    Err(_) => {}
                }
                writer.flush_written()?;
                flushed = true;
            }
            blocks.commit(&plan);
        }
        Ok(())
    }
    fn batch_interval(&self) -> Option<Duration> {
        Some(BATCH_INTERVAL)
    }
    /// Flushes aged data even while no blocks arrive.
    fn poll(&mut self) -> Result<()> {
        if let Some(writer) = self.0.as_mut() {
            writer.poll(Instant::now())?;
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        if let Some(writer) = self.0.take() {
            writer.finish()?;
        }
        Ok(())
    }
}
