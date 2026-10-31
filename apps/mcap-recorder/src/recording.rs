use crate::config::{
    self, Input, RecorderConfig, RecordingReport, State, Validation, pb::InputReport,
};
use dora_node_api::{Event, Metadata, Parameter, StopCause};
use eyre::{Result, bail, ensure, eyre};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Seek, SeekFrom, Write},
};
use telemetry_mcap::RecordingSink;

struct Channel {
    input: Input,
    channel: u16,
    report_index: usize,
}

pub struct Recording {
    sink: RecordingSink,
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
        let sink = RecordingSink::create(&config.output_path)?;
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
            sink,
            channels: BTreeMap::new(),
            report,
            report_file,
        };
        for (input, bytes) in prepared {
            let cpu = input.validation != Validation::RawBytes as i32;
            let schema = if cpu {
                Some((
                    telemetry_protocol::MCAP_SCHEMA_NAME,
                    "protobuf",
                    telemetry_protocol::FILE_DESCRIPTOR_SET,
                ))
            } else {
                bytes.as_deref().map(|bytes| {
                    (
                        input.schema_name.as_str(),
                        input.schema_encoding.as_str(),
                        bytes,
                    )
                })
            };
            let metadata = BTreeMap::from([
                ("dora.input".into(), input.input.clone()),
                (
                    "timing".into(),
                    if cpu {
                        "validated_cpu_capture"
                    } else {
                        "log=receipt_unix_ns;publish=configured_source_or_zero"
                    }
                    .into(),
                ),
                ("source_timestamp_key".into(), input.timestamp_key.clone()),
                ("source_sequence_key".into(), input.sequence_key.clone()),
            ]);
            let channel = recording.sink.add_channel(
                &input.topic,
                &input.message_encoding,
                schema,
                &metadata,
            )?;
            let index = recording.report.input.len();
            recording.report.input.push(InputReport {
                input: input.input.clone(),
                ..Default::default()
            });
            recording.channels.insert(
                input.input.clone(),
                Channel {
                    input,
                    channel,
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
        let (sequence, log_time, publish_time) =
            match Validation::try_from(channel.input.validation)? {
                Validation::CpuSnapshot => {
                    let frame = telemetry_protocol::decode(bytes)?;
                    ensure!(
                        decimal(metadata, "sequence")? == frame.snapshot.sequence,
                        "payload sequence differs from transport metadata"
                    );
                    ensure!(
                        decimal(metadata, "capture_unix_ns")? == frame.snapshot.capture_unix_ns,
                        "payload capture time differs from transport metadata"
                    );
                    report.producer_dropped_batches = Some(frame.snapshot.dropped_batches);
                    report.producer_capacity_overflow = Some(frame.snapshot.capacity_overflow);
                    (
                        frame.snapshot.sequence as u32,
                        frame.snapshot.capture_unix_ns,
                        frame.snapshot.capture_unix_ns,
                    )
                }
                Validation::CpuDictionary => {
                    let frame = telemetry_protocol::decode_dictionary(bytes)?;
                    (
                        frame.revision as u32,
                        frame.capture_unix_ns,
                        frame.capture_unix_ns,
                    )
                }
                Validation::RawBytes => {
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
                    (sequence, received_ns, source_time)
                }
            };
        // Ordinal is recorder-local correlation, never a manufactured source sequence.
        self.sink.metadata(
            "dora.message",
            BTreeMap::from([
                ("input".into(), input.into()),
                ("ordinal".into(), report.received.to_string()),
                ("receipt_unix_ns".into(), received_ns.to_string()),
                ("dora_metadata_json".into(), metadata_json(metadata)?),
            ]),
        )?;
        self.sink
            .write(channel.channel, sequence, log_time, publish_time, bytes)?;
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
        match self.sink.finish() {
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

fn metadata_json(metadata: &Metadata) -> Result<String> {
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
    Ok(serde_json::to_string(metadata)?)
}

fn decimal(metadata: &Metadata, key: &str) -> Result<u64> {
    match metadata.parameters.get(key) {
        Some(Parameter::String(value)) => Ok(value.parse()?),
        _ => bail!("missing decimal string metadata: {key}"),
    }
}

/// Drain buffered events after AllInputsClosed; explicit Stop leaves incomplete evidence.
pub fn consume(
    recording: &mut Recording,
    events: impl Iterator<Item = Event>,
) -> (State, Result<()>) {
    let mut closed = BTreeSet::new();
    let mut all_closed = false;
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
                all_closed = false;
                Ok(())
            }
            Event::Stop(StopCause::AllInputsClosed) => {
                all_closed = true;
                Ok(())
            }
            Event::Stop(_) => {
                return (
                    State::Interrupted,
                    Err(eyre!("recording interrupted before input drain")),
                );
            }
            Event::Error(error) => Err(eyre!("Dora event error: {error}")),
            Event::NodeFailed { error, .. } => Err(eyre!("upstream node failed: {error}")),
            _ => Ok(()),
        };
        if let Err(error) = result {
            return (State::Failed, Err(error));
        }
    }
    if all_closed || recording.channels.keys().all(|id| closed.contains(id)) {
        (State::Drained, Ok(()))
    } else {
        (
            State::Interrupted,
            Err(eyre!("Dora stream ended without confirmed input closure")),
        )
    }
}
