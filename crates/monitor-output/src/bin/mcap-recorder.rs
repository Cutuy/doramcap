//! A separate Dora process; collection never calls the MCAP writer.
use dora_node_api::{
    arrow::array::{Array, UInt8Array},
    DoraNode, Event, Parameter,
};
use eyre::{bail, eyre, Result};
use monitor_output::McapSink;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or_else(|| eyre!("usage: mcap-recorder OUTPUT.mcap"))?;
    if args.next().is_some() {
        bail!("usage: mcap-recorder OUTPUT.mcap");
    }
    let (_node, mut events) = DoraNode::init_from_env_force()?;
    let mut sink = McapSink::create(path)?;
    let result = (|| -> Result<()> {
        while let Some(event) = events.recv() {
            match event {
                Event::Input { id, data, metadata } if id.as_str() == "snapshots" => {
                    let read_u64 = |key: &str| -> Result<u64> {
                        match metadata.parameters.get(key) {
                            Some(Parameter::String(value)) => Ok(value.parse()?),
                            _ => bail!("missing decimal string metadata: {key}"),
                        }
                    };
                    let sequence = read_u64("sequence")?;
                    let capture = read_u64("capture_unix_ns")?;
                    let bytes = data
                        .as_any()
                        .downcast_ref::<UInt8Array>()
                        .ok_or_else(|| eyre!("snapshot must be UInt8 JSON bytes"))?;
                    if bytes.null_count() != 0 {
                        bail!("snapshot contains null bytes");
                    }
                    sink.write(sequence, capture, bytes.values())?;
                }
                Event::InputClosed { id } if id.as_str() == "snapshots" => break,
                Event::Stop(_) => break,
                Event::Error(error) => bail!("Dora event error: {error}"),
                Event::NodeFailed { error, .. } => bail!("upstream node failed: {error}"),
                _ => {}
            }
        }
        Ok(())
    })();
    // Preserve a readable partial recording even when the upstream node fails.
    sink.finish()?;
    result
}
