//! A separate Dora process; collection never calls the MCAP writer.
use dora_node_api::{
    Event, Parameter, StopCause,
    futures::{StreamExt, executor::block_on},
};
use eyre::{Result, bail, eyre};
use telemetry_mcap::McapSink;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or_else(|| eyre!("usage: mcap-recorder OUTPUT.mcap"))?;
    if args.next().is_some() {
        bail!("usage: mcap-recorder OUTPUT.mcap");
    }
    let (_node, mut events) = dora_app::environment::connect()?;
    let mut sink = McapSink::create(path)?;
    dora_app::environment::ready()?;
    let result = (|| -> Result<()> {
        while let Some(event) = block_on(events.next()) {
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
                    if data.null_count() != 0 {
                        bail!("snapshot contains null bytes");
                    }
                    let bytes: &[u8] = (&data).try_into()?;
                    sink.write(sequence, capture, bytes)?;
                }
                Event::Input { id, data, .. } if id.as_str() == "dictionary" => {
                    if data.null_count() != 0 {
                        bail!("dictionary contains null bytes");
                    }
                    let bytes: &[u8] = (&data).try_into()?;
                    sink.write_dictionary(bytes)?;
                }
                Event::InputClosed { .. } | Event::Stop(StopCause::AllInputsClosed) => {}
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
