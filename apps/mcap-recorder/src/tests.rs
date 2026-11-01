use crate::{
    config::{self, Input, RecorderConfig, RecordingReport, State},
    recording::{Recording, consume},
};
use dora_node_api::{Event, IntoArrow, Metadata, Parameter, StopCause};
use eyre::Result;
use prost::Message;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "configured-recorder-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
    fn config(&self) -> RecorderConfig {
        RecorderConfig {
            output_path: self.0.join("test.mcap").to_str().unwrap().into(),
            report_path: self.0.join("report.pbtxt").to_str().unwrap().into(),
            input: vec![Input {
                input: "data".into(),
                topic: "/test".into(),
                message_encoding: "application/octet-stream".into(),
                ..Default::default()
            }],
        }
    }
    fn report(&self) -> Result<RecordingReport> {
        let pool = prost_reflect::DescriptorPool::decode(
            include_bytes!(concat!(env!("OUT_DIR"), "/recorder.bin")).as_slice(),
        )?;
        Ok(prost_reflect::DynamicMessage::parse_text_format(
            pool.get_message_by_name("doramcap.recorder.v1.RecordingReport")
                .unwrap(),
            &std::fs::read_to_string(self.0.join("report.pbtxt"))?,
        )?
        .transcode_to()?)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn metadata() -> Metadata {
    Metadata::new(dora_node_api::uhlc::HLC::default().new_timestamp())
}
fn event(input: &str, bytes: &[u8], metadata: Metadata) -> Event {
    Event::Input {
        id: input.into(),
        data: bytes.to_vec().into_arrow(),
        metadata,
    }
}

#[test]
fn configured_topics_keep_schema_bytes_source_and_receipt_times() -> Result<()> {
    // Arrange
    let temp = Temp::new()?;
    let schema_path = temp.0.join("schema.json");
    std::fs::write(&schema_path, br#"{"type":"object"}"#)?;
    let mut config = temp.config();
    config.input[0].message_encoding = "json".into();
    config.input[0].schema_name = "Object".into();
    config.input[0].schema_encoding = "jsonschema".into();
    config.input[0].schema_path = schema_path.to_str().unwrap().into();
    config.input[0].timestamp_key = "capture".into();
    config.input[0].sequence_key = "seq".into();
    config.input.push(Input {
        input: "status".into(),
        topic: "/status".into(),
        message_encoding: "text/plain".into(),
        ..Default::default()
    });
    let path = config.output_path.clone();
    let mut recording = Recording::create(config)?;
    let mut source = metadata();
    source
        .parameters
        .insert("capture".into(), Parameter::String("123".into()));
    source
        .parameters
        .insert("seq".into(), Parameter::String("7".into()));
    source
        .parameters
        .insert("label".into(), Parameter::String("source".into()));
    // Act
    recording.record("data", b"{}", &source, 456)?;
    recording.record("status", b"ready", &metadata(), 789)?;
    recording.finish(State::Drained, Ok(()))?;
    // Assert
    let bytes = std::fs::read(path)?;
    let messages = mcap::MessageStream::new(&bytes)?.collect::<std::result::Result<Vec<_>, _>>()?;
    assert_eq!(messages.len(), 2);
    assert_eq!(
        (
            messages[0].sequence,
            messages[0].log_time,
            messages[0].publish_time
        ),
        (7, 456, 123)
    );
    assert_eq!(messages[0].channel.topic, "/test");
    assert_eq!(messages[0].data.as_ref(), b"{}");
    assert_eq!(
        messages[0].channel.schema.as_ref().unwrap().data.as_ref(),
        br#"{"type":"object"}"#
    );
    assert_eq!(
        (
            messages[1].sequence,
            messages[1].log_time,
            messages[1].publish_time
        ),
        (0, 789, 0)
    );
    assert_eq!(messages[1].channel.topic, "/status");
    assert_eq!(messages[1].data.as_ref(), b"ready");
    assert!(
        bytes
            .windows(b"dora_metadata_json".len())
            .any(|v| v == b"dora_metadata_json")
    );
    let report = temp.report()?;
    assert_eq!(report.state, State::Drained as i32);
    assert!(report.finalized && report.transport_loss_unknown);
    assert!(report.input.iter().all(|input| input.written == 1));
    Ok(())
}

#[test]
fn all_inputs_closed_drains_buffered_tail_but_unconfirmed_end_is_interrupted() -> Result<()> {
    // Arrange
    let temp = Temp::new()?;
    let config = temp.config();
    let path = config.output_path.clone();
    let mut recording = Recording::create(config)?;
    // Act: Dora can deliver the Stop notification before its buffered tail.
    let (state, result) = consume(
        &mut recording,
        vec![
            Event::Stop(StopCause::AllInputsClosed),
            event("data", b"tail", metadata()),
        ]
        .into_iter(),
    );
    recording.finish(state, result)?;
    // Assert
    let bytes = std::fs::read(path)?;
    assert_eq!(mcap::MessageStream::new(&bytes)?.count(), 1);
    assert_eq!(temp.report()?.state, State::Drained as i32);

    // Arrange
    let incomplete = Temp::new()?;
    let mut recording = Recording::create(incomplete.config())?;
    // Act
    let (state, result) = consume(
        &mut recording,
        vec![event("data", b"partial", metadata())].into_iter(),
    );
    assert!(recording.finish(state, result).is_err());
    // Assert
    let report = incomplete.report()?;
    assert_eq!(report.state, State::Interrupted as i32);
    assert!(report.finalized);
    assert_eq!(report.input[0].written, 1);
    Ok(())
}

#[test]
fn invalid_config_and_existing_outputs_reject_before_recording() -> Result<()> {
    // Arrange
    let temp = Temp::new()?;
    let valid = temp.config();
    let mut invalid = valid.clone();
    invalid.input.push(invalid.input[0].clone());
    // Act / Assert
    assert!(Recording::create(invalid).is_err());
    assert!(!std::path::Path::new(&valid.output_path).exists());
    std::fs::write(&valid.output_path, b"existing")?;
    assert!(Recording::create(valid.clone()).is_err());
    assert_eq!(std::fs::read(valid.output_path)?, b"existing");
    Ok(())
}

#[test]
fn unsupported_payload_or_upstream_error_keeps_partial_failure_evidence() -> Result<()> {
    // Arrange
    let temp = Temp::new()?;
    let mut recording = Recording::create(temp.config())?;
    let unsupported = Event::Input {
        id: "data".into(),
        data: vec![1_i64].into_arrow(),
        metadata: metadata(),
    };
    // Act
    let (state, result) = consume(&mut recording, vec![unsupported].into_iter());
    assert!(recording.finish(state, result).is_err());
    // Assert
    let report = temp.report()?;
    assert_eq!(report.state, State::Failed as i32);
    assert!(report.finalized);
    assert!(report.error.contains("uint8"));

    // Arrange
    let partial = Temp::new()?;
    let mut recording = Recording::create(partial.config())?;
    // Act
    let (state, result) = consume(
        &mut recording,
        vec![
            event("data", b"good", metadata()),
            Event::Error("source failed".into()),
        ]
        .into_iter(),
    );
    assert!(recording.finish(state, result).is_err());
    // Assert
    let report = partial.report()?;
    assert!(report.finalized && report.error.contains("source failed"));
    assert_eq!(report.input[0].written, 1);
    Ok(())
}

#[test]
fn cpu_validation_preserves_capture_and_reports_producer_loss() -> Result<()> {
    // Arrange
    let temp = Temp::new()?;
    let mut config = config::cpu(temp.0.join("test.mcap").to_str().unwrap().into());
    config.report_path = temp.0.join("report.pbtxt").to_str().unwrap().into();
    let mut snapshot = monitor_core::Snapshot::with_capacity(1);
    snapshot.sequence = 17;
    snapshot.capture_unix_ns = 123;
    snapshot.dropped_batches = 4;
    snapshot.capacity_overflow = 2;
    let frame = telemetry_protocol::Frame {
        version: telemetry_protocol::VERSION,
        snapshot,
        source: "fixture".into(),
        task_collection_enabled: false,
        task_capacity: 0,
        session_id: "fixture".into(),
        dictionary_revision: 1,
        dictionary_full: false,
        dictionary_overflow: 0,
        dictionary: Vec::<telemetry_protocol::DictionaryEntry>::new(),
    };
    let payload = telemetry_protocol::snapshot_packet(&frame).encode_to_vec();
    let mut metadata = metadata();
    metadata
        .parameters
        .insert("sequence".into(), Parameter::String("17".into()));
    metadata
        .parameters
        .insert("capture_unix_ns".into(), Parameter::String("123".into()));
    let mut recording = Recording::create(config)?;
    // Act
    recording.record("snapshots", &payload, &metadata, 456)?;
    metadata
        .parameters
        .insert("sequence".into(), Parameter::String("18".into()));
    assert!(
        recording
            .record("snapshots", &payload, &metadata, 456)
            .is_err()
    );
    assert!(
        recording
            .finish(State::Failed, Err(eyre::eyre!("invalid source metadata")))
            .is_err()
    );
    // Assert
    let report = temp.report()?;
    assert_eq!(report.input[0].producer_dropped_batches, Some(4));
    assert_eq!(report.input[0].producer_capacity_overflow, Some(2));
    assert_eq!((report.input[0].received, report.input[0].written), (2, 1));
    Ok(())
}

#[test]
fn pbtxt_paths_resolve_relative_to_config_and_invalid_settings_fail() -> Result<()> {
    // Arrange
    let temp = Temp::new()?;
    let path = temp.0.join("recorder.pbtxt");
    std::fs::write(
        &path,
        "output_path: 'session.mcap' input { input: 'data' topic: '/test' message_encoding: 'json' schema_name: 'Example' schema_encoding: 'jsonschema' schema_path: 'schema.json' }",
    )?;
    std::fs::write(temp.0.join("schema.json"), b"{}")?;
    // Act
    let config = config::load(&path)?;
    // Assert
    assert_eq!(
        std::path::Path::new(&config.output_path),
        temp.0.join("session.mcap")
    );
    assert_eq!(
        std::path::Path::new(&config.input[0].schema_path),
        temp.0.join("schema.json")
    );
    // Arrange
    std::fs::write(
        &path,
        "output_path: 'session.mcap' input { input: 'data' topic: '/test' message_encoding: 'json' schema_name: 'Incomplete' }",
    )?;
    // Act / Assert
    assert!(config::load(&path).is_err());
    assert!(!temp.0.join("session.mcap").exists());
    Ok(())
}

#[test]
fn workflow_stop_drains_buffered_tail_and_validates_declared_source_time() -> Result<()> {
    // Arrange
    let temp = Temp::new()?;
    let mut recording = Recording::create(temp.config())?;
    // Act
    let (state, result) = consume(
        &mut recording,
        vec![
            event("data", b"partial", metadata()),
            Event::Stop(StopCause::Manual),
            event("data", b"tail", metadata()),
        ]
        .into_iter(),
    );
    recording.finish(state, result)?;
    // Assert
    let report = temp.report()?;
    assert_eq!(report.state, State::Drained as i32);
    assert!(report.finalized);
    assert_eq!(report.input[0].written, 2);
    let bytes = std::fs::read(temp.config().output_path)?;
    let messages = mcap::MessageStream::new(&bytes)?.collect::<std::result::Result<Vec<_>, _>>()?;
    assert_eq!(messages[1].data.as_ref(), b"tail");
    // Arrange
    let bad = Temp::new()?;
    let mut config = bad.config();
    config.input[0].timestamp_key = "required_source_time".into();
    let mut recording = Recording::create(config)?;
    // Act
    let (state, result) = consume(
        &mut recording,
        vec![event("data", b"payload", metadata())].into_iter(),
    );
    assert!(recording.finish(state, result).is_err());
    // Assert
    assert!(bad.report()?.error.contains("required_source_time"));
    assert_eq!(bad.report()?.input[0].written, 0);
    Ok(())
}

#[test]
fn unknown_inputs_and_unrepresentable_metadata_fail_explicitly() -> Result<()> {
    // Arrange
    let temp = Temp::new()?;
    let mut recording = Recording::create(temp.config())?;
    // Act
    let (state, result) = consume(
        &mut recording,
        vec![event("unconfigured", b"bytes", metadata())].into_iter(),
    );
    assert!(recording.finish(state, result).is_err());
    // Assert
    assert!(temp.report()?.error.contains("unconfigured"));
    // Arrange
    let bad = Temp::new()?;
    let mut recording = Recording::create(bad.config())?;
    let mut invalid = metadata();
    invalid
        .parameters
        .insert("unrepresentable".into(), Parameter::Float(f64::NAN));
    // Act
    let (state, result) = consume(
        &mut recording,
        vec![event("data", b"bytes", invalid)].into_iter(),
    );
    assert!(recording.finish(state, result).is_err());
    // Assert
    assert!(bad.report()?.error.contains("non-finite"));
    assert_eq!(bad.report()?.input[0].written, 0);
    Ok(())
}
