use eyre::{Result, ensure};
use prost_reflect::{DescriptorPool, DynamicMessage};
use std::{collections::BTreeSet, path::Path};

pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.recorder.v1.rs"));
}
pub use pb::{Input, RecorderConfig, RecordingReport, input::Validation, recording_report::State};
const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/recorder.bin"));

/// Loads pbtxt; relative schema, recording and report paths use its directory.
pub fn load(path: &Path) -> Result<RecorderConfig> {
    let descriptor = DescriptorPool::decode(DESCRIPTOR)?
        .get_message_by_name("doramcap.recorder.v1.RecorderConfig")
        .expect("compiled configuration");
    let mut config: RecorderConfig =
        DynamicMessage::parse_text_format(descriptor, &std::fs::read_to_string(path)?)?
            .transcode_to()?;
    let directory = path.parent().unwrap_or(Path::new("."));
    let resolve = |value: &mut String| -> Result<()> {
        if !value.is_empty() {
            *value = directory
                .join(&*value)
                .to_str()
                .ok_or_else(|| eyre::eyre!("path is not UTF-8"))?
                .into();
        }
        Ok(())
    };
    resolve(&mut config.output_path)?;
    resolve(&mut config.report_path)?;
    for input in &mut config.input {
        resolve(&mut input.schema_path)?;
    }
    validate(&config)?;
    Ok(config)
}

/// Rejects ambiguous mappings and incomplete schema or CPU validation settings.
pub fn validate(config: &RecorderConfig) -> Result<()> {
    ensure!(!config.output_path.is_empty(), "output_path is required");
    ensure!(
        config.report_path.is_empty() || config.report_path != config.output_path,
        "report_path must differ from output_path"
    );
    ensure!(!config.input.is_empty(), "at least one input is required");
    let mut names = BTreeSet::new();
    let mut topics = BTreeSet::new();
    for input in &config.input {
        ensure!(
            !input.input.is_empty() && names.insert(&input.input),
            "empty or duplicate input: {}",
            input.input
        );
        ensure!(
            !input.topic.is_empty() && topics.insert(&input.topic),
            "empty or duplicate topic: {}",
            input.topic
        );
        ensure!(
            !input.message_encoding.is_empty(),
            "{}: message_encoding is required",
            input.input
        );
        let validation = Validation::try_from(input.validation)?;
        let fields = [
            &input.schema_name,
            &input.schema_encoding,
            &input.schema_path,
        ];
        ensure!(
            fields.iter().all(|s| s.is_empty()) || fields.iter().all(|s| !s.is_empty()),
            "{}: schema_name, schema_encoding and schema_path must be supplied together",
            input.input
        );
        if validation != Validation::RawBytes {
            ensure!(
                input.message_encoding == "protobuf" && fields.iter().all(|s| s.is_empty()),
                "{}: CPU inputs use protobuf and the embedded telemetry schema",
                input.input
            );
            ensure!(
                input.timestamp_key.is_empty() && input.sequence_key.is_empty(),
                "{}: CPU timing and sequence come from the validated payload",
                input.input
            );
        }
    }
    Ok(())
}

pub fn cpu(output_path: String) -> RecorderConfig {
    RecorderConfig {
        output_path,
        report_path: String::new(),
        input: vec![
            Input {
                input: "snapshots".into(),
                topic: "doramcap/snapshots".into(),
                message_encoding: "protobuf".into(),
                validation: Validation::CpuSnapshot as i32,
                ..Default::default()
            },
            Input {
                input: "dictionary".into(),
                topic: "doramcap/dictionary".into(),
                message_encoding: "protobuf".into(),
                validation: Validation::CpuDictionary as i32,
                ..Default::default()
            },
        ],
    }
}

pub fn report_text(report: &RecordingReport) -> Result<String> {
    let descriptor = DescriptorPool::decode(DESCRIPTOR)?
        .get_message_by_name("doramcap.recorder.v1.RecordingReport")
        .expect("compiled report");
    let mut value = DynamicMessage::new(descriptor);
    value.transcode_from(report)?;
    Ok(value.to_text_format())
}
