use crate::{BatchView, Blocks, Result, Sink, SinkBuilder, wire};
use std::{
    io,
    path::{Path, PathBuf},
    time::Instant,
};
use storage_persistence::mcap::{Channel, McapConfig, McapWriter, Message, Schema};

const CHANNEL: u16 = 1;

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
            for attempt in 0..2 {
                match writer.reserve(message, plan.len) {
                    Ok(mut reservation) => {
                        blocks.encode(reservation.payload_mut());
                        reservation.try_commit()?;
                        break;
                    }
                    // Retry once after draining; retained registrations can still refuse capacity.
                    Err(error) if attempt == 1 || error.kind() != io::ErrorKind::WouldBlock => {
                        return Err(error.into());
                    }
                    Err(_) => {}
                }
                writer.flush_written()?;
            }
            blocks.commit(&plan);
        }
        Ok(())
    }
    fn batch_blocks(&self) -> bool {
        true
    }
    fn next_flush_deadline(&self) -> Option<Instant> {
        self.0.as_ref().and_then(McapWriter::next_flush_deadline)
    }
    /// Flushes buffered data at the writer's deadline even while no blocks arrive.
    fn poll(&mut self) -> Result<()> {
        if let Some(writer) = self.0.as_mut() {
            let now = Instant::now();
            if writer
                .next_flush_deadline()
                .is_some_and(|deadline| now >= deadline)
            {
                writer.flush_written()?;
            } else {
                writer.poll(now)?;
            }
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
