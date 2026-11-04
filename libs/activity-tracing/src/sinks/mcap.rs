use crate::{BatchView, Result, Sink, SinkBuilder, wire};
use std::path::{Path, PathBuf};
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
    fn write(&mut self, b: BatchView<'_>) -> Result<()> {
        let message = Message {
            channel: CHANNEL,
            sequence: b.sequence as u32,
            log_time: b.unix_ns,
            publish_time: b.unix_ns,
        };
        let writer = self.0.as_mut().ok_or("MCAP finalized")?;
        writer.write(message, b.bytes)?;
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        if let Some(writer) = self.0.take() {
            writer.finish()?;
        }
        Ok(())
    }
}
