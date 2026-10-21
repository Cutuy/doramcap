//! Typed runtime configuration. Includes/overrides preserve scalar presence.
use pbtxt_config::{Error, Payload, Result};
pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.watchdog.v1.rs"));
}
pub use pb::WatchdogConfig;
fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(message.into()))
    }
}
impl Payload for WatchdogConfig {
    const NAME: &'static str = "doramcap.watchdog.v1.WatchdogConfig";
    const DESCRIPTOR: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
    fn validate_partial(&self) -> Result<()> {
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
                value.is_none_or(|v| v > 0 && v <= max),
                &format!("invalid {name}"),
            )?;
        }
        require(
            self.retry_count.is_none_or(|v| v <= 10),
            "retry_count exceeds 10",
        )?;
        require(
            self.transport_allowance_ms.is_none_or(|v| v <= 60000),
            "transport allowance exceeds 60s",
        )?;
        require(
            self.inputs.len() <= 1023
                && self.inputs.iter().all(|v| !v.is_empty() && v.len() <= 128),
            "invalid inputs",
        )?;
        let unique: std::collections::BTreeSet<_> = self.inputs.iter().collect();
        require(unique.len() == self.inputs.len(), "duplicate inputs")?;
        require(
            self.live_output
                .as_ref()
                .is_none_or(|s| !s.is_empty() && s.len() <= 128),
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
    fn overlay(&self, target: &mut Self) -> Vec<&'static str> {
        let mut changed = Vec::new();
        if !self.inputs.is_empty() {
            target.inputs = self.inputs.clone();
            changed.push("inputs");
        }
        macro_rules! copy { ($($f:ident),*) => { $(if self.$f.is_some(){target.$f.clone_from(&self.$f);changed.push(stringify!($f));})* }; }
        copy!(
            max_sources,
            max_conditions_per_source,
            max_batch_records,
            output_capacity,
            dedup_window,
            transport_allowance_ms,
            source_timeout_ms,
            snapshot_interval_ms,
            maintenance_ms,
            max_frame_bytes,
            queue_capacity,
            shutdown_ms,
            retry_count,
            retry_delay_ms,
            live_output,
            mcap,
            otel
        );
        changed
    }
    fn normalize(&self) -> Result<Self> {
        self.validate_partial()?;
        let mut v = self.clone();
        macro_rules! default { ($($f:ident=$n:expr),*) => { $(v.$f.get_or_insert($n);)* }; }
        default!(
            max_sources = 64,
            max_conditions_per_source = 256,
            max_batch_records = 512,
            output_capacity = 1024,
            dedup_window = 128,
            transport_allowance_ms = 0,
            source_timeout_ms = 5000,
            snapshot_interval_ms = 1000,
            maintenance_ms = 10,
            max_frame_bytes = 2 * 1024 * 1024,
            queue_capacity = 32,
            shutdown_ms = 5000,
            retry_count = 2,
            retry_delay_ms = 100
        );
        v.live_output.get_or_insert_with(|| "faults".into());
        require(!v.inputs.is_empty(), "at least one input is required")?;
        require(
            v.max_sources.unwrap() as usize > v.inputs.len(),
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
            (v.inputs.len() as u64 + 1) * u64::from(v.max_conditions_per_source.unwrap());
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
        Ok(v)
    }
}
/// Resolve the `watchdog` app from a TextFormat fragment, validating requested features.
pub fn load(path: &std::path::Path) -> Result<WatchdogConfig> {
    pbtxt_config::resolve::<WatchdogConfig>(path, &["watchdog"])?.config("watchdog")
}
