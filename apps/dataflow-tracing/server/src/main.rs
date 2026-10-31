mod monitor;
mod pipeline;
use dora_node_api::{DoraNode, MetadataParameters};
use eyre::{Result, bail};
use prost::Message;
#[allow(clippy::large_enum_variant)] // Generated protobuf wire representation.
pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.dataflow_tracing.v1.rs"));
}
pub const MAX_FRAME: usize = 1 << 20;
pub fn publish(node: &mut DoraNode, topic: &str, message: &impl Message) -> Result<()> {
    let bytes = message.encode_to_vec();
    eyre::ensure!(bytes.len() <= MAX_FRAME, "oversized publication");
    node.send_output_raw(
        topic.into(),
        MetadataParameters::default(),
        bytes.len(),
        |out| out.copy_from_slice(&bytes),
    )?;
    Ok(())
}
pub fn node_id(name: &str) -> String {
    format!(
        "{}-{name}",
        std::env::var("PROVENANCE_PREFIX").unwrap_or_else(|_| "provenance".into())
    )
}
pub fn envelope(body: pb::envelope::Body) -> pb::Envelope {
    pb::Envelope {
        version: 1,
        sequence: 0,
        stream_id: String::new(),
        body: Some(body),
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("pipeline") => pipeline::run(&args[1..]),
        Some("monitor") => monitor::run(&args[1..]).await,
        _ => bail!(
            "usage: dataflow-tracing pipeline [--duration N] [--drop-every N] [--report-delay-ms N] | monitor [--port N] [--assets PATH] [--measurements PATH]"
        ),
    }
}
