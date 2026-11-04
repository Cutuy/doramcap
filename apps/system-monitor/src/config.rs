//! Launch configuration: generated `MonitorConfig` types loaded from pbtxt.
use eyre::{Result, ensure, eyre};
use pbtxt_config::Payload;
use std::path::Path;

pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.monitor.v1.rs"));
}
pub use pb::{MonitorConfig, Recording};

const MAX_ACQUISITION_BYTES: usize = 512 * 1024 * 1024;
/// The app ID of this config in a pbtxt fragment.
pub const APP: &str = "monitor";

fn invalid(error: eyre::Report) -> pbtxt_config::Error {
    pbtxt_config::Error::Invalid(error.to_string())
}
impl Payload for MonitorConfig {
    const NAME: &'static str = "doramcap.monitor.v1.MonitorConfig";
    const DESCRIPTOR: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/monitor.bin"));
    fn validate_partial(&self) -> pbtxt_config::Result<()> {
        validate_assigned(self).map_err(invalid)
    }
    fn overlay(&self, target: &mut Self) -> Vec<&'static str> {
        let mut changed = Vec::new();
        macro_rules! assign { ($($f:ident),*) => { $(if pbtxt_config::assign(&self.$f, &mut target.$f) { changed.push(stringify!($f)); })* }; }
        assign!(
            records,
            queue_slots,
            task_capacity,
            samples,
            output_path,
            published_count_path,
            dora,
            recording
        );
        changed
    }
    fn normalize(&self) -> pbtxt_config::Result<Self> {
        validate(self).map_err(invalid)?;
        Ok(self.clone())
    }
}

/// Loads pbtxt and its includes, resolves relative paths beside `path` and validates the launch.
/// Errors: unreadable or malformed files and any `validate` failure.
pub fn load(path: &Path) -> Result<MonitorConfig> {
    let mut config = pbtxt_config::resolve::<MonitorConfig>(path, &[APP])?.config(APP)?;
    let directory = path.parent().unwrap_or(Path::new("."));
    let resolve = |value: &mut String| -> Result<()> {
        if !value.is_empty() {
            *value = directory
                .join(&*value)
                .to_str()
                .ok_or_else(|| eyre!("path is not UTF-8"))?
                .into();
        }
        Ok(())
    };
    resolve(&mut config.output_path)?;
    resolve(&mut config.published_count_path)?;
    if let Some(recording) = config.recording.as_mut() {
        resolve(&mut recording.path)?;
    }
    validate(&config)?;
    Ok(config)
}

/// Rejects assigned values outside documented bounds and incomplete recording sections.
fn validate_assigned(config: &MonitorConfig) -> Result<()> {
    ensure!(
        config.records <= 1_048_576
            && config.queue_slots <= 1024
            && config.task_capacity() <= 1_048_576,
        "invalid capacities: records 1..1048576, queue_slots 1..1024, task_capacity 0..1048576"
    );
    if let Some(recording) = &config.recording {
        ensure!(!recording.path.is_empty(), "recording path is required");
        ensure!(
            !recording.samples_topic.is_empty()
                && !recording.dictionary_topic.is_empty()
                && recording.samples_topic != recording.dictionary_topic,
            "recording topics must be nonempty and distinct"
        );
    }
    Ok(())
}

/// Rejects incomplete launches, storage over budget and outputs that share one path.
pub fn validate(config: &MonitorConfig) -> Result<()> {
    validate_assigned(config)?;
    ensure!(
        config.records >= 1 && config.queue_slots >= 1,
        "invalid capacities: records 1..1048576, queue_slots 1..1024, task_capacity 0..1048576"
    );
    let bytes = (config.records as usize)
        .checked_mul(std::mem::size_of::<monitor_core::CpuRecord>())
        .and_then(|n| {
            (config.task_capacity() as usize)
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
        ensure!(
            recording.path != config.output_path,
            "recording path must differ from output_path"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = env!("CARGO_MANIFEST_DIR");

    fn resolved_text(path: &Path) -> pbtxt_config::Result<String> {
        pbtxt_config::resolve::<MonitorConfig>(path, &[APP])?.fragment_text()
    }

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
    fn a_profile_overrides_its_base_and_resolves_to_one_complete_file() {
        // Arrange: the release profile adds outputs to its base; an overlay may clear diagnostics.
        let record = format!("{ROOT}/../../console/release/system-apps/cpu-monitor/monitor.pbtxt");
        let directory =
            std::env::temp_dir().join(format!("monitor-resolved-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let base = directory.join("base.pbtxt");
        let overlay = directory.join("overlay.pbtxt");
        let entry = |fields: &str| {
            format!(
                "app {{ id: 'monitor' config {{ [type.googleapis.com/doramcap.monitor.v1.MonitorConfig] {{ {fields} }} }} }}"
            )
        };
        std::fs::write(&base, entry("records: 1 queue_slots: 1 task_capacity: 8")).unwrap();
        std::fs::write(
            &overlay,
            format!("include: 'base.pbtxt' {}", entry("task_capacity: 0")),
        )
        .unwrap();

        // Act
        let inherited = load(Path::new(&record)).unwrap();
        let cleared = load(&overlay).unwrap();
        let text = resolved_text(Path::new(&record)).unwrap();
        let alone = directory.join("alone.pbtxt");
        std::fs::write(&alone, &text).unwrap();
        let resolved = load(&alone).unwrap();
        let again = resolved_text(&alone).unwrap();
        std::fs::remove_dir_all(&directory).unwrap();

        // Assert
        assert_eq!(inherited.records, 4096);
        assert!(inherited.dora.is_some() && inherited.recording.is_some());
        assert_eq!(inherited.output_path, "/results/cpu.pb");
        assert_eq!(cleared.task_capacity(), 0);
        assert_eq!(resolved, inherited);
        assert_eq!(again, text);
    }

    #[test]
    fn relative_paths_resolve_beside_the_config() {
        // Arrange
        let path = format!("{ROOT}/deploy/supervisor/monitor.pbtxt");
        let directory = Path::new(&path).parent().unwrap();

        // Act
        let config = load(Path::new(&path)).unwrap();

        // Assert
        assert_eq!(Path::new(&config.output_path), directory.join("cpu.pb"));
        assert_eq!(
            Path::new(&config.recording.unwrap().path),
            directory.join("cpu.mcap")
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
            edit(|c| c.task_capacity = Some(1_048_577)),
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
