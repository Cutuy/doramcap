//! Launch configuration: generated `MonitorConfig` types loaded from pbtxt.
use eyre::{Result, ensure, eyre};
use prost_reflect::{DescriptorPool, DynamicMessage};
use std::path::Path;

pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.monitor.v1.rs"));
}
pub use pb::{MonitorConfig, Recording};

const MAX_ACQUISITION_BYTES: usize = 512 * 1024 * 1024;

/// Parses standard TextFormat and validates the supplied settings.
/// Errors: unreadable or malformed files and any `validate` failure.
pub fn load(path: &Path) -> Result<MonitorConfig> {
    let descriptor = DescriptorPool::decode(
        include_bytes!(concat!(env!("OUT_DIR"), "/monitor.bin")).as_slice(),
    )?
    .get_message_by_name("doramcap.monitor.v1.MonitorConfig")
    .expect("compiled configuration");
    let config: MonitorConfig =
        DynamicMessage::parse_text_format(descriptor, &std::fs::read_to_string(path)?)?
            .transcode_to()?;
    validate(&config)?;
    Ok(config)
}

/// Rejects capacities outside documented bounds and incomplete recording sections.
pub fn validate(config: &MonitorConfig) -> Result<()> {
    ensure!(
        (1..=1_048_576).contains(&config.records)
            && (1..=1024).contains(&config.queue_slots)
            && config.task_capacity <= 1_048_576,
        "invalid capacities: records 1..1048576, queue_slots 1..1024, task_capacity 0..1048576"
    );
    let bytes = (config.records as usize)
        .checked_mul(std::mem::size_of::<monitor_core::CpuRecord>())
        .and_then(|n| {
            (config.task_capacity as usize)
                .checked_mul(std::mem::size_of::<task_accounting::TaskRecord>())
                .and_then(|t| n.checked_add(t))
        })
        .and_then(|n| n.checked_mul(config.queue_slots as usize + 2))
        .ok_or_else(|| eyre!("capacity overflow"))?;
    ensure!(
        bytes <= MAX_ACQUISITION_BYTES,
        "configured acquisition storage exceeds 512 MiB; reduce capacities"
    );
    if let Some(recording) = &config.recording {
        ensure!(!recording.path.is_empty(), "recording path is required");
        ensure!(
            recording.path != config.output_path,
            "recording path must differ from output_path"
        );
        ensure!(
            !recording.samples_topic.is_empty()
                && !recording.dictionary_topic.is_empty()
                && recording.samples_topic != recording.dictionary_topic,
            "recording topics must be nonempty and distinct"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = env!("CARGO_MANIFEST_DIR");

    #[test]
    fn shipped_launches_select_their_documented_outputs() {
        // Arrange
        let load_shipped = |path: String| load(Path::new(&path)).unwrap();

        // Act
        let dora_only = load_shipped(format!("{ROOT}/../cpu-viewer/dataflows/live-monitor.pbtxt"));
        let stream_only = load_shipped(format!("{ROOT}/deploy/monitor.pbtxt"));
        let recording_only = load_shipped(format!("{ROOT}/deploy/recording.pbtxt"));
        let combined = load_shipped(format!("{ROOT}/../cpu-viewer/deploy/monitor.pbtxt"));

        // Assert
        assert!(dora_only.dora.is_some() && dora_only.recording.is_none());
        assert!(stream_only.dora.is_none() && stream_only.recording.is_none());
        assert_eq!(stream_only.output_path, "/results/monitor.pb");
        assert!(recording_only.dora.is_none() && recording_only.recording.is_some());
        assert!(combined.dora.is_some() && combined.recording.is_some());
        assert_eq!(combined.published_count_path, "/results/published.count");
    }

    #[test]
    fn paths_preserve_protobuf_values() {
        // Arrange
        let path = format!("{ROOT}/deploy/supervisor/monitor.pbtxt");

        // Act
        let config = load(Path::new(&path)).unwrap();

        // Assert
        assert_eq!(Path::new(&config.output_path), Path::new("cpu.pb"));
        assert_eq!(
            Path::new(&config.recording.unwrap().path),
            Path::new("cpu.mcap")
        );
    }

    #[test]
    fn invalid_launches_are_rejected() {
        // Arrange
        let valid = MonitorConfig {
            records: 1,
            queue_slots: 1,
            recording: Some(Recording {
                path: "/results/cpu.mcap".into(),
                samples_topic: "samples".into(),
                dictionary_topic: "dictionary".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let edit = |change: fn(&mut MonitorConfig)| {
            let mut config = valid.clone();
            change(&mut config);
            config
        };

        // Act
        let invalid = [
            edit(|c| c.records = 0),
            edit(|c| c.records = 1_048_577),
            edit(|c| c.queue_slots = 0),
            edit(|c| c.task_capacity = 1_048_577),
            edit(|c| {
                c.records = 1_048_576;
                c.queue_slots = 1024;
            }),
            edit(|c| c.output_path = "/results/cpu.mcap".into()),
            edit(|c| c.recording.as_mut().unwrap().path.clear()),
            edit(|c| c.recording.as_mut().unwrap().samples_topic.clear()),
            edit(|c| c.recording.as_mut().unwrap().dictionary_topic = "samples".into()),
        ];

        // Assert
        assert!(validate(&valid).is_ok());
        for config in invalid {
            assert!(validate(&config).is_err(), "{config:?}");
        }
    }
}
