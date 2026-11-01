//! Dynamic Dora participant used by shared-runtime integration tests.
use dora_node_api::{
    DoraNode, Event, StopCause,
    dora_core::config::NodeId,
    futures::{StreamExt, executor::block_on},
};
use eyre::{Result, bail, eyre};
use prost::Message;
use std::{io::Write, os::unix::fs::OpenOptionsExt};
use telemetry_mcap::RecordingSink;

#[path = "../src/config.rs"]
pub mod config;

fn acknowledge(path: &str) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(b"ready\n")?;
    file.sync_all()?;
    Ok(())
}

fn shared_mapping(bytes: &[u8]) -> Result<bool> {
    let address = bytes.as_ptr() as usize;
    Ok(std::fs::read_to_string("/proc/self/maps")?
        .lines()
        .any(|line| {
            let Some((range, _)) = line.split_once(' ') else {
                return false;
            };
            let Some((start, end)) = range.split_once('-') else {
                return false;
            };
            let (Ok(start), Ok(end)) = (
                usize::from_str_radix(start, 16),
                usize::from_str_radix(end, 16),
            ) else {
                return false;
            };
            start <= address && address + bytes.len() <= end && line.contains("/dev/shm/")
        }))
}

fn finish_recording(sink: RecordingSink, count: u64, proof: Option<&str>) -> Result<()> {
    if count != 20 {
        bail!("expected 20 messages, received {count}");
    }
    sink.finish()?;
    if let Some(proof) = proof {
        acknowledge(proof)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let node = args.next().ok_or_else(|| eyre!("missing node ID"))?;
    let port = args
        .next()
        .ok_or_else(|| eyre!("missing daemon port"))?
        .parse()?;
    let ready = args.next().ok_or_else(|| eyre!("missing readiness path"))?;
    let mode = args.next().unwrap_or_else(|| "idle".into());
    let path = args.next();
    let done = args.next();
    if args.next().is_some() {
        bail!("unexpected argument");
    }
    let (mut node, mut events) = DoraNode::builder()
        .node_id(NodeId::from(node))
        .daemon_port(port)
        .build()?;
    let mut recorder = if mode == "recorder" {
        let config = config::load(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/deploy/reusable/cpu.pbtxt"
        )))?;
        let input = config
            .input
            .iter()
            .find(|input| input.validation == config::Validation::CpuSnapshot as i32)
            .ok_or_else(|| eyre!("CPU snapshot channel missing from fixture config"))?;
        let mut sink = RecordingSink::create(
            path.as_deref()
                .ok_or_else(|| eyre!("missing recording path"))?,
        )?;
        let channel = sink.add_channel(
            &input.topic,
            &input.message_encoding,
            Some((
                telemetry_protocol::MCAP_SCHEMA_NAME,
                "protobuf",
                telemetry_protocol::FILE_DESCRIPTOR_SET,
            )),
            &Default::default(),
        )?;
        Some((sink, channel))
    } else {
        None
    };
    acknowledge(&ready)?;
    if mode == "producer" {
        let bytes = std::fs::read(
            path.as_deref()
                .ok_or_else(|| eyre!("missing payload path"))?,
        )?;
        let mut packet = telemetry_protocol::pb::Packet::decode(bytes.as_slice())?;
        for sequence in 0..20 {
            let Some(telemetry_protocol::pb::packet::Body::Snapshot(frame)) = packet.body.as_mut()
            else {
                bail!("expected a snapshot fixture");
            };
            let snapshot = frame
                .snapshot
                .as_mut()
                .ok_or_else(|| eyre!("missing snapshot"))?;
            snapshot.sequence = sequence;
            let capture_unix_ns = snapshot.capture_unix_ns;
            let bytes = packet.encode_to_vec();
            let mut parameters = dora_node_api::MetadataParameters::default();
            parameters.insert(
                "sequence".into(),
                dora_node_api::Parameter::String(sequence.to_string()),
            );
            parameters.insert(
                "capture_unix_ns".into(),
                dora_node_api::Parameter::String(capture_unix_ns.to_string()),
            );
            node.send_output_raw("data".into(), parameters, bytes.len(), |output| {
                output.copy_from_slice(&bytes)
            })?;
        }
        acknowledge(
            done.as_deref()
                .ok_or_else(|| eyre!("missing producer completion path"))?,
        )?;
    } else if mode != "idle" && mode != "recorder" {
        bail!("invalid fixture mode");
    }
    let require_shm = std::env::var_os("DORAMCAP_REQUIRE_SHM").is_some();
    let mut count = 0u64;
    while let Some(event) = block_on(events.next()) {
        match event {
            Event::Input { data, metadata, .. } if recorder.is_some() => {
                let bytes: &[u8] = (&data).try_into()?;
                if require_shm && !shared_mapping(bytes)? {
                    bail!("received payload is not backed by /dev/shm");
                }
                let parameter = |key: &str| -> Result<u64> {
                    match metadata.parameters.get(key) {
                        Some(dora_node_api::Parameter::String(value)) => Ok(value.parse()?),
                        _ => bail!("missing {key} metadata"),
                    }
                };
                let (sink, channel) = recorder.as_mut().expect("recorder");
                sink.write(
                    *channel,
                    u32::try_from(parameter("sequence")?)?,
                    parameter("capture_unix_ns")?,
                    parameter("capture_unix_ns")?,
                    bytes,
                )?;
                count += 1;
            }
            Event::InputClosed { .. } | Event::Stop(StopCause::AllInputsClosed) => {}
            Event::Stop(_) => break,
            Event::Error(error) => bail!("Dora event error: {error}"),
            Event::NodeFailed { error, .. } => bail!("upstream node failed: {error}"),
            _ => {}
        }
    }
    if let Some((sink, _)) = recorder {
        let proof = require_shm
            .then(|| {
                done.as_deref()
                    .ok_or_else(|| eyre!("missing SHM proof path"))
            })
            .transpose()?;
        finish_recording(sink, count, proof)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_recordings_cannot_publish_shm_success() -> Result<()> {
        // Arrange
        let root = std::env::temp_dir().join(format!(
            "dora-fixture-proof-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir(&root)?;
        for count in [0, 19] {
            let sink = RecordingSink::create(root.join(format!("{count}.mcap")))?;
            let proof = root.join(format!("{count}.proof"));
            // Act
            let result = finish_recording(sink, count, Some(proof.to_str().unwrap()));
            // Assert
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains(&format!("received {count}"))
            );
            assert!(!proof.exists());
        }
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
}
