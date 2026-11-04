//! Standard WatchdogConfig TextFormat parsing and launch validation.
use eyre::{Result, ensure};
use prost_reflect::{DescriptorPool, DynamicMessage};
use std::time::Duration;
pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.watchdog.v1.rs"));
}
pub use pb::WatchdogConfig;
fn require(ok: bool, message: &str) -> Result<()> {
    ensure!(ok, "{message}");
    Ok(())
}
impl WatchdogConfig {
    pub fn validate(&self) -> Result<()> {
        for (name, value, max) in [
            ("max_sources", self.max_sources, 1024),
            (
                "max_conditions_per_source",
                self.max_conditions_per_source,
                4096,
            ),
            ("max_batch_records", self.max_batch_records, 32768),
            ("output_capacity", self.output_capacity, 32768),
            ("dedup_window", self.dedup_window, 4096),
            ("source_timeout_ms", self.source_timeout_ms, 3600000),
            ("snapshot_interval_ms", self.snapshot_interval_ms, 60000),
            ("maintenance_ms", self.maintenance_ms, 1000),
            ("max_frame_bytes", self.max_frame_bytes, 8 * 1024 * 1024),
            ("queue_capacity", self.queue_capacity, 1024),
            ("shutdown_ms", self.shutdown_ms, 60000),
            ("retry_delay_ms", self.retry_delay_ms, 10000),
        ] {
            require(
                value.is_some_and(|v| v > 0 && v <= max),
                &format!("invalid {name}"),
            )?;
        }
        require(
            self.retry_count.is_some_and(|v| v <= 10),
            "retry_count exceeds 10",
        )?;
        require(
            self.transport_allowance_ms.is_some_and(|v| v <= 60000),
            "transport allowance exceeds 60s",
        )?;
        require(
            self.live_output
                .as_ref()
                .is_some_and(|s| !s.is_empty() && s.len() <= 128),
            "invalid live_output",
        )?;
        if let Some(m) = &self.mcap {
            require(
                !m.enabled || cfg!(feature = "mcap"),
                "mcap requested but not compiled",
            )?;
            require(
                !m.enabled
                    || (!m.path.is_empty()
                        && m.path.len() <= 4096
                        && m.flush_ms > 0
                        && m.flush_ms <= 60000),
                "invalid MCAP config",
            )?;
        }
        if let Some(o) = &self.otel {
            require(
                !o.enabled || cfg!(feature = "otel"),
                "otel requested but not compiled",
            )?;
            require(
                !o.enabled
                    || ((o.endpoint.starts_with("http://") || o.endpoint.starts_with("https://"))
                        && o.endpoint.len() <= 2048
                        && o.timeout_ms > 0
                        && o.timeout_ms <= 30000
                        && !o.service_name.is_empty()
                        && o.service_name.len() <= 192),
                "invalid OTel config",
            )?;
        }
        Ok(())
    }
    fn validate_inputs(&self, inputs: &[String]) -> Result<()> {
        self.validate()?;
        require(
            inputs.len() <= 1023 && inputs.iter().all(|v| !v.is_empty() && v.len() <= 128),
            "invalid inputs",
        )?;
        let unique: std::collections::BTreeSet<_> = inputs.iter().collect();
        require(unique.len() == inputs.len(), "duplicate inputs")?;
        let v = self;
        require(
            v.max_sources.unwrap() as usize > inputs.len(),
            "max_sources must include all inputs and local diagnostics",
        )?;
        require(
            v.source_timeout_ms.unwrap() > v.maintenance_ms.unwrap(),
            "source timeout must exceed maintenance interval",
        )?;
        require(
            u64::from(v.max_frame_bytes.unwrap()) * u64::from(v.queue_capacity.unwrap())
                <= 64 * 1024 * 1024,
            "per-sink queue budget exceeds 64 MiB",
        )?;
        require(
            u64::from(v.max_sources.unwrap()) * u64::from(v.max_conditions_per_source.unwrap())
                <= 16384,
            "condition budget exceeds 16384",
        )?;
        let conditions =
            (inputs.len() as u64 + 1) * u64::from(v.max_conditions_per_source.unwrap());
        require(
            v.max_conditions_per_source.unwrap() >= 4,
            "reserve four host diagnostic conditions",
        )?;
        require(
            conditions * 2048 + 1024 <= u64::from(v.max_frame_bytes.unwrap()),
            "frame budget cannot hold worst-case snapshot",
        )?;
        require(
            u64::from(v.max_batch_records.unwrap()) * 2048 + 1024
                <= u64::from(v.max_frame_bytes.unwrap()),
            "frame budget cannot hold worst-case batch",
        )?;
        Ok(())
    }
}
impl WatchdogConfig {
    /// Validates explicit settings and Dora inputs before constructing the engine.
    pub fn settings(&self, inputs: Vec<String>) -> Result<fault_reconciliation::Settings> {
        self.validate_inputs(&inputs)?;
        let c = self;
        let ms = |v: Option<u32>| Duration::from_millis(v.unwrap().into());
        Ok(fault_reconciliation::Settings {
            aggregation: fault_aggregation::Config {
                max_sources: c.max_sources.unwrap() as usize,
                max_conditions_per_source: c.max_conditions_per_source.unwrap() as usize,
                max_batch_records: c.max_batch_records.unwrap() as usize,
                output_capacity: c.output_capacity.unwrap() as usize,
                dedup_window: c.dedup_window.unwrap() as usize,
                transport_allowance: ms(c.transport_allowance_ms),
            },
            source_timeout: ms(c.source_timeout_ms),
            snapshot_interval: ms(c.snapshot_interval_ms),
            maintenance: ms(c.maintenance_ms),
            max_frame_bytes: c.max_frame_bytes.unwrap() as usize,
            inputs,
        })
    }
}
/// Parses one WatchdogConfig TextFormat file and validates explicit settings.
pub fn load(path: &std::path::Path) -> Result<WatchdogConfig> {
    let descriptor =
        DescriptorPool::decode(include_bytes!(concat!(env!("OUT_DIR"), "/config.bin")).as_slice())?
            .get_message_by_name("doramcap.watchdog.v1.WatchdogConfig")
            .expect("compiled configuration");
    let config: WatchdogConfig =
        DynamicMessage::parse_text_format(descriptor, &std::fs::read_to_string(path)?)?
            .transcode_to()?;
    config.validate()?;
    Ok(config)
}
