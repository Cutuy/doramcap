use crate::config::{
    self, Input, METADATA_TOPIC_SUFFIX, RecorderConfig, RecordingReport, State, pb::InputReport,
};
use dora_node_api::{Event, Metadata, Parameter};
use eyre::{Result, bail, ensure, eyre};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Seek, SeekFrom, Write},
};
use storage_persistence::mcap::{McapConfig, McapWriter, Message};

struct Channel {
    input: Input,
    channel: u16,
    metadata_channel: u16,
    report_index: usize,
}

pub struct Recording {
    writer: McapWriter,
    channels: BTreeMap<String, Channel>,
    report: RecordingReport,
    report_file: File,
}

fn save_report(file: &mut File, report: &RecordingReport) -> Result<()> {
    let text = config::report_text(report)?;
    file.seek(SeekFrom::Start(0))?;
    file.set_len(0)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

impl Recording {
    /// Validates configuration and schemas before creating output and report files.
    pub fn create(config: RecorderConfig) -> Result<Self> {
        config::validate(&config)?;
        let mut prepared = Vec::new();
        for input in config.input {
            let bytes = if input.schema_path.is_empty() {
                None
            } else {
                Some(std::fs::read(&input.schema_path)?)
            };
            prepared.push((input, bytes));
        }
        let report_path = if config.report_path.is_empty() {
            format!("{}.report.pbtxt", config.output_path)
        } else {
            config.report_path
        };
        let writer = create_writer(&config.output_path, &prepared)?;
        let mut report_file = File::options()
            .write(true)
            .create_new(true)
            .open(report_path)?;
        let report = RecordingReport {
            transport_loss_unknown: true,
            ..Default::default()
        };
        save_report(&mut report_file, &report)?;
        let mut recording = Self {
            writer,
            channels: BTreeMap::new(),
            report,
            report_file,
        };
        for (index, (input, _)) in prepared.into_iter().enumerate() {
            let (channel, metadata_channel) = channel_ids(index)?;
            recording.report.input.push(InputReport {
                input: input.input.clone(),
                ..Default::default()
            });
            recording.channels.insert(
                input.input.clone(),
                Channel {
                    input,
                    channel,
                    metadata_channel,
                    report_index: index,
                },
            );
        }
        save_report(&mut recording.report_file, &recording.report)?;
        Ok(recording)
    }

    /// Stores uint8 bytes and typed Dora metadata; unknown inputs fail explicitly.
    pub fn record(
        &mut self,
        input: &str,
        bytes: &[u8],
        metadata: &Metadata,
        received_ns: u64,
    ) -> Result<()> {
        let channel = self
            .channels
            .get(input)
            .ok_or_else(|| eyre!("unconfigured Dora input: {input}"))?;
        let report = &mut self.report.input[channel.report_index];
        report.received += 1;
        let sequence = if channel.input.sequence_key.is_empty() {
            0
        } else {
            u32::try_from(decimal(metadata, &channel.input.sequence_key)?)?
        };
        let source_time = if channel.input.timestamp_key.is_empty() {
            0
        } else {
            decimal(metadata, &channel.input.timestamp_key)?
        };
        let message = |channel| Message {
            channel,
            sequence,
            log_time: received_ns,
            publish_time: source_time,
        };
        // Ordinal is recorder-local correlation, never a manufactured source sequence.
        self.writer.write(
            message(channel.metadata_channel),
            &metadata_json(report.received, metadata)?,
        )?;
        self.writer.write(message(channel.channel), bytes)?;
        report.written += 1;
        Ok(())
    }

    /// Finalizes readable partial output and reports drain/interruption independently.
    pub fn finish(mut self, state: State, result: Result<()>) -> Result<()> {
        self.report.state = state as i32;
        let mut errors = Vec::new();
        if let Err(error) = result {
            errors.push(error.to_string());
        }
        match self.writer.finish() {
            Ok(()) => self.report.finalized = true,
            Err(error) => {
                self.report.state = State::Failed as i32;
                errors.push(format!("MCAP finalization: {error}"));
            }
        }
        self.report.error = errors.join("; ");
        if let Err(error) = save_report(&mut self.report_file, &self.report) {
            errors.push(format!("recording report: {error}"));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            bail!("{}", errors.join("; "))
        }
    }
}

/// Payload and companion metadata channel IDs for one input's position.
fn channel_ids(index: usize) -> Result<(u16, u16)> {
    let channel = u16::try_from(index * 2 + 1)?;
    Ok((
        channel,
        channel
            .checked_add(1)
            .ok_or_else(|| eyre!("too many inputs"))?,
    ))
}

fn string(out: &mut Vec<u8>, value: &str) -> Result<()> {
    out.extend_from_slice(&u32::try_from(value.len())?.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

/// Appends one framed MCAP record.
fn record(out: &mut Vec<u8>, opcode: u8, content: &[u8]) {
    out.push(opcode);
    out.extend_from_slice(&(content.len() as u64).to_le_bytes());
    out.extend_from_slice(content);
}

/// Appends a framed MCAP Channel record; schema ID zero means no schema.
fn channel_record(
    out: &mut Vec<u8>,
    (id, schema): (u16, u16),
    topic: &str,
    encoding: &str,
    metadata: &BTreeMap<&str, &str>,
) -> Result<()> {
    let mut map = Vec::new();
    for (key, value) in metadata {
        string(&mut map, key)?;
        string(&mut map, value)?;
    }
    let mut content = Vec::new();
    content.extend_from_slice(&id.to_le_bytes());
    content.extend_from_slice(&schema.to_le_bytes());
    string(&mut content, topic)?;
    string(&mut content, encoding)?;
    content.extend_from_slice(&u32::try_from(map.len())?.to_le_bytes());
    content.extend_from_slice(&map);
    record(out, 4, &content);
    Ok(())
}

/// Creates the recording exclusively with direct I/O and declares each input's channels.
fn create_writer(path: &str, inputs: &[(Input, Option<Vec<u8>>)]) -> Result<McapWriter> {
    let mut records = Vec::new();
    for (index, (input, schema)) in inputs.iter().enumerate() {
        let (channel, metadata_channel) = channel_ids(index)?;
        let schema_id = if let Some(data) = schema {
            let id = u16::try_from(index + 1)?;
            let mut content = id.to_le_bytes().to_vec();
            string(&mut content, &input.schema_name)?;
            string(&mut content, &input.schema_encoding)?;
            content.extend_from_slice(&u32::try_from(data.len())?.to_le_bytes());
            content.extend_from_slice(data);
            record(&mut records, 3, &content);
            id
        } else {
            0
        };
        channel_record(
            &mut records,
            (channel, schema_id),
            &input.topic,
            &input.message_encoding,
            &BTreeMap::from([
                ("dora.input", input.input.as_str()),
                (
                    "timing",
                    "log=receipt_unix_ns;publish=configured_source_or_zero",
                ),
                ("source_timestamp_key", input.timestamp_key.as_str()),
                ("source_sequence_key", input.sequence_key.as_str()),
            ]),
        )?;
        channel_record(
            &mut records,
            (metadata_channel, 0),
            &format!("{}{METADATA_TOPIC_SUFFIX}", input.topic),
            "json",
            &BTreeMap::from([("dora.input", input.input.as_str())]),
        )?;
    }
    Ok(McapWriter::with_records(
        path,
        &records,
        McapConfig {
            buffers: 8,
            buffer_size: 256 * 1024,
            ..Default::default()
        },
    )?)
}

fn metadata_json(ordinal: u64, metadata: &Metadata) -> Result<Vec<u8>> {
    for (key, value) in &metadata.parameters {
        let finite = match value {
            Parameter::Float(value) => value.is_finite(),
            Parameter::ListFloat(values) => values.iter().all(|value| value.is_finite()),
            _ => true,
        };
        ensure!(
            finite,
            "{key}: non-finite Dora metadata cannot be preserved as JSON"
        );
    }
    Ok(serde_json::to_vec(&serde_json::json!({
        "ordinal": ordinal,
        "metadata": metadata,
    }))?)
}

fn decimal(metadata: &Metadata, key: &str) -> Result<u64> {
    match metadata.parameters.get(key) {
        Some(Parameter::String(value)) => Ok(value.parse()?),
        _ => bail!("missing decimal string metadata: {key}"),
    }
}

/// Drain buffered events after Dora stop; unconfirmed stream loss leaves incomplete evidence.
pub fn consume(
    recording: &mut Recording,
    events: impl Iterator<Item = Event>,
) -> (State, Result<()>) {
    let mut closed = BTreeSet::new();
    let mut stopped = false;
    for event in events {
        let result = match event {
            Event::Input { id, data, metadata } => (|| -> Result<()> {
                let received_ns = u64::try_from(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_nanos(),
                )?;
                ensure!(data.null_count() == 0, "{id} contains null bytes");
                let bytes: &[u8] = (&data)
                    .try_into()
                    .map_err(|error| eyre!("{id}: only uint8 payloads are supported: {error}"))?;
                recording.record(id.as_str(), bytes, &metadata, received_ns)
            })(),
            Event::InputClosed { id } => {
                closed.insert(id.to_string());
                Ok(())
            }
            Event::InputRecovered { id } => {
                closed.remove(id.as_str());
                Ok(())
            }
            Event::Stop(_) => {
                // Dora ends the stream after delivering buffered events for every stop cause.
                stopped = true;
                Ok(())
            }
            Event::Error(error) => Err(eyre!("Dora event error: {error}")),
            Event::NodeFailed { error, .. } => Err(eyre!("upstream node failed: {error}")),
            _ => Ok(()),
        };
        if let Err(error) = result {
            return (State::Failed, Err(error));
        }
    }
    if stopped || recording.channels.keys().all(|id| closed.contains(id)) {
        (State::Drained, Ok(()))
    } else {
        (
            State::Interrupted,
            Err(eyre!("Dora stream ended without confirmed input closure")),
        )
    }
}
