use crate::{BatchView, Result, Sink, wire};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
#[cfg(feature = "websocket")]
mod websocket;
#[cfg(feature = "websocket")]
pub use websocket::WebSocketSink;
#[cfg(feature = "otel")]
mod otel;
#[cfg(feature = "otel")]
pub use otel::{OtelSink, OtelStats};

/// Embedded synchronous MCAP writer, exclusively owned by an output worker.
pub struct McapSink {
    writer: Option<mcap::Writer<BufWriter<File>>>,
    channel: u16,
}
impl McapSink {
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::options().write(true).create_new(true).open(path)?;
        let mut writer = mcap::Writer::new(BufWriter::new(file))?;
        let schema = writer.add_schema(wire::SCHEMA, "protobuf", wire::DESCRIPTOR)?;
        let channel =
            writer.add_channel(schema, "doramcap/activity", "protobuf", &BTreeMap::new())?;
        Ok(Self {
            writer: Some(writer),
            channel,
        })
    }
}
impl Sink for McapSink {
    fn name(&self) -> &str {
        "mcap"
    }
    fn write(&mut self, b: BatchView<'_>) -> Result<()> {
        self.writer
            .as_mut()
            .ok_or("MCAP finalized")?
            .write_to_known_channel(
                &mcap::records::MessageHeader {
                    channel_id: self.channel,
                    sequence: b.sequence as u32,
                    log_time: b.unix_ns,
                    publish_time: b.unix_ns,
                },
                b.bytes,
            )?;
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        if let Some(mut w) = self.writer.take() {
            w.finish()?;
            w.into_inner().flush()?;
        }
        Ok(())
    }
}
