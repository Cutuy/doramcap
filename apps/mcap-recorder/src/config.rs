use eyre::{Result, ensure};
use pbtxt_config::Payload;
use prost_reflect::{DescriptorPool, DynamicMessage};
use std::{collections::BTreeSet, path::Path};

pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.recorder.v1.rs"));
}
pub use pb::{Input, RecorderConfig, RecordingReport, recording_report::State};
/// Each input's `<topic><suffix>` channel holds per-message Dora metadata.
pub const METADATA_TOPIC_SUFFIX: &str = "/dora_metadata";
const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/recorder.bin"));
/// The app ID of this config in a pbtxt fragment.
pub const APP: &str = "recorder";

fn invalid(error: eyre::Report) -> pbtxt_config::Error {
    pbtxt_config::Error::Invalid(error.to_string())
}
impl Payload for RecorderConfig {
    const NAME: &'static str = "doramcap.recorder.v1.RecorderConfig";
    const DESCRIPTOR: &'static [u8] = DESCRIPTOR;
    fn validate_partial(&self) -> pbtxt_config::Result<()> {
        validate_inputs(self).map_err(invalid)
    }
    fn overlay(&self, target: &mut Self) -> Vec<&'static str> {
        let mut changed = Vec::new();
        macro_rules! assign { ($($f:ident),*) => { $(if pbtxt_config::assign(&self.$f, &mut target.$f) { changed.push(stringify!($f)); })* }; }
        assign!(output_path, report_path, input);
        changed
    }
    fn normalize(&self) -> pbtxt_config::Result<Self> {
        validate(self).map_err(invalid)?;
        Ok(self.clone())
    }
}

/// Loads pbtxt and its includes; relative schema, recording and report paths use `path`'s directory.
pub fn load(path: &Path) -> Result<RecorderConfig> {
    let mut config = pbtxt_config::resolve::<RecorderConfig>(path, &[APP])?.config(APP)?;
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

/// Rejects incomplete recorders, ambiguous mappings and incomplete schema settings.
pub fn validate(config: &RecorderConfig) -> Result<()> {
    ensure!(!config.output_path.is_empty(), "output_path is required");
    ensure!(
        config.report_path.is_empty() || config.report_path != config.output_path,
        "report_path must differ from output_path"
    );
    ensure!(!config.input.is_empty(), "at least one input is required");
    validate_inputs(config)
}

/// Rejects ambiguous mappings and incomplete schema settings among assigned inputs.
fn validate_inputs(config: &RecorderConfig) -> Result<()> {
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
            !input.topic.ends_with(METADATA_TOPIC_SUFFIX),
            "{}: topic suffix {METADATA_TOPIC_SUFFIX} is reserved for recorded Dora metadata",
            input.input
        );
        ensure!(
            !input.message_encoding.is_empty(),
            "{}: message_encoding is required",
            input.input
        );
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
        // Readers cannot decode a protobuf channel without its schema.
        ensure!(
            input.message_encoding != "protobuf" || !input.schema_path.is_empty(),
            "{}: message_encoding protobuf requires a schema",
            input.input
        );
    }
    Ok(())
}

pub fn report_text(report: &RecordingReport) -> Result<String> {
    let descriptor = DescriptorPool::decode(DESCRIPTOR)?
        .get_message_by_name("doramcap.recorder.v1.RecordingReport")
        .expect("compiled report");
    let mut value = DynamicMessage::new(descriptor);
    value.transcode_from(report)?;
    Ok(value.to_text_format())
}
