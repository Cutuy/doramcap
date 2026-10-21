//! Single-owner host state, with explicit ingress and timer advancement.
use crate::config::WatchdogConfig;
use eyre::{Result, ensure};
use fault_aggregation::{Aggregator, FaultObservability};
use fault_protocol::{AggregationFrame, AggregationPayload, Limits, ReporterPayload, Updates};
use fault_reporter::{Check, Definition, Publisher, Reporter, Text};
use pbtxt_config::Payload;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
struct Input {
    source: Option<u128>,
    seen: Duration,
    connected: bool,
}
/// Owns source bindings, local diagnostics and aggregation. All times passed to
/// methods share one monotonic epoch and must be nondecreasing. No I/O occurs.
pub struct Engine {
    config: WatchdogConfig,
    pub observability: Arc<FaultObservability>,
    aggregator: Aggregator,
    inputs: BTreeMap<String, Input>,
    reporter: Reporter,
    publisher: Publisher,
    ingress: Check,
    next_snapshot: Duration,
    epoch: u128,
}
impl Engine {
    /// Validate configuration and reserve a separate source for host diagnostics.
    /// `epoch` and `source` must be distinct nonzero incarnation identifiers.
    pub fn new(config: WatchdogConfig, epoch: u128, source: u128) -> Result<Self> {
        ensure!(epoch != source, "host identities must differ");
        let config = config.normalize()?;
        let observability = Arc::new(FaultObservability::new());
        let aggregator = Aggregator::new(
            epoch,
            fault_aggregation::Config {
                max_sources: config.max_sources.unwrap() as usize,
                max_conditions_per_source: config.max_conditions_per_source.unwrap() as usize,
                max_batch_records: config.max_batch_records.unwrap() as usize,
                output_capacity: config.output_capacity.unwrap() as usize,
                dedup_window: config.dedup_window.unwrap() as usize,
                transport_allowance: ms(config.transport_allowance_ms.unwrap()),
            },
            observability.clone(),
        )?;
        let (reporter, publisher) = Reporter::new(source, Default::default())?;
        let ingress =
            reporter.register(Definition::new("watchdog", "ingress", "invalid_report")?)?;
        let inputs = config
            .inputs
            .iter()
            .map(|s| {
                (
                    s.clone(),
                    Input {
                        source: None,
                        seen: Duration::ZERO,
                        connected: false,
                    },
                )
            })
            .collect();
        Ok(Self {
            config,
            observability,
            aggregator,
            inputs,
            reporter,
            publisher,
            ingress,
            next_snapshot: Duration::ZERO,
            epoch,
        })
    }
    /// Register a local sink-health condition; retain its key for self-feedback filtering.
    pub fn sink_check(&self, name: &str) -> Result<Check> {
        let mut d = Definition::new("watchdog", name, "publication_failed")?;
        d.policy.success_interval = Some(Duration::from_secs(1));
        Ok(self.reporter.register(d)?)
    }
    /// Frame/record limits shared by ingress and publication adapters.
    pub fn limits(&self) -> Limits {
        Limits {
            max_frame_bytes: self.config.max_frame_bytes.unwrap() as usize,
            max_records: 32768,
        }
    }
    /// Decode one configured input. A registration snapshot starts each incarnation.
    /// Errors preserve any accepted prefix and activate the ingress diagnostic.
    /// A new incarnation retires the old one; source IDs cannot be shared by inputs.
    pub fn receive(&mut self, input: &str, bytes: &[u8], now: Duration) -> Result<()> {
        let result = self.receive_inner(input, bytes, now);
        match &result {
            Ok(()) => {
                self.ingress.success()?;
            }
            Err(e) => {
                self.reject(input, e, now)?;
            }
        }
        result
    }
    /// Report a transport/decoding failure and mark the affected input stale.
    /// The error is bounded to the reporter's fixed-size UTF-8 detail field.
    pub fn reject(
        &mut self,
        input: &str,
        error: &impl std::fmt::Display,
        now: Duration,
    ) -> Result<()> {
        self.ingress.failure(detail(error))?;
        self.disconnect(input, now)
    }
    fn receive_inner(&mut self, input: &str, bytes: &[u8], now: Duration) -> Result<()> {
        let frame = fault_protocol::decode_reporter(bytes, self.limits())?;
        let source = match &frame.payload {
            ReporterPayload::Snapshot(s) => s.source,
            ReporterPayload::Batch(b) => b.source,
        };
        let binding = self
            .inputs
            .get(input)
            .ok_or_else(|| eyre::eyre!("unconfigured input"))?;
        ensure!(
            source != self.ingress.key().source,
            "source collides with local diagnostics"
        );
        ensure!(
            !self
                .inputs
                .iter()
                .any(|(id, i)| id != input && i.source == Some(source)),
            "source shared by inputs"
        );
        let old = binding.source;
        if old != Some(source) {
            ensure!(
                matches!(frame.payload, ReporterPayload::Snapshot(_)),
                "snapshot required before batches"
            );
            if let Some(old) = old {
                match self.aggregator.set_connected(old, false, now) {
                    Ok(()) => self.aggregator.retire_source(old)?,
                    Err(fault_aggregation::Error::UnknownSource) => {}
                    Err(e) => return Err(e.into()),
                }
            }
            // Reserve binding before reconciliation: even a rejected partial snapshot
            // remains owned by this input and can be repaired or retired on restart.
            self.inputs.get_mut(input).unwrap().source = Some(source);
        } else {
            match self.aggregator.set_connected(source, true, now) {
                Ok(()) | Err(fault_aggregation::Error::UnknownSource) => {}
                Err(e) => return Err(e.into()),
            }
        }
        match frame.payload {
            ReporterPayload::Snapshot(s) => self.aggregator.reconcile(&s, now)?,
            ReporterPayload::Batch(b) => self.aggregator.ingest_batch(&b, now)?,
        }
        let i = self.inputs.get_mut(input).unwrap();
        i.seen = now;
        i.connected = true;
        Ok(())
    }
    /// Mark a closed input stale while retaining its faults for inspection.
    pub fn disconnect(&mut self, input: &str, now: Duration) -> Result<()> {
        if let Some(i) = self.inputs.get_mut(input) {
            if let Some(source) = i.source {
                match self.aggregator.set_connected(source, false, now) {
                    Ok(()) | Err(fault_aggregation::Error::UnknownSource) => {}
                    Err(e) => return Err(e.into()),
                }
            }
            i.connected = false;
        }
        Ok(())
    }
    /// Advance transport/freshness deadlines and ingest local diagnostics.
    pub fn advance(&mut self, now: Duration) -> Result<()> {
        let timeout = ms(self.config.source_timeout_ms.unwrap());
        for i in self.inputs.values_mut() {
            if i.connected && now.saturating_sub(i.seen) >= timeout {
                self.aggregator
                    .set_connected(i.source.unwrap(), false, now)?;
                i.connected = false;
            }
        }
        if let Some(snapshot) = self.publisher.take_reconciliation() {
            self.aggregator.reconcile(&snapshot, now)?;
        }
        let batch = self
            .publisher
            .batch(self.config.max_batch_records.unwrap() as usize)?;
        if !batch.observations.is_empty() {
            self.aggregator.ingest_batch(&batch, now)?;
        }
        self.aggregator.advance(now)?;
        Ok(())
    }
    /// Drain currently ready updates without waiting to fill a batch, followed by
    /// an overflow or periodic snapshot. Encoding failures must remain visible to the host.
    pub fn publications(&mut self, now: Duration) -> Vec<AggregationFrame> {
        let mut frames = Vec::new();
        while self.aggregator.has_updates() {
            let mut updates = Vec::new();
            for _ in 0..self.config.max_batch_records.unwrap() {
                if let Some(u) = self.aggregator.pop_update() {
                    updates.push(u)
                } else {
                    break;
                }
            }
            frames.push(self.frame(
                AggregationPayload::Updates(Updates {
                    epoch: self.epoch,
                    updates,
                    stats: self.observability.stats(),
                }),
                now,
            ));
        }
        let reconcile = self.aggregator.take_reconciliation().is_some();
        if reconcile || now >= self.next_snapshot {
            frames.push(self.snapshot(now));
            self.next_snapshot = now + ms(self.config.snapshot_interval_ms.unwrap());
        }
        frames
    }
    /// Capture the authoritative current view for reconnect/queue-loss repair.
    pub fn snapshot(&self, now: Duration) -> AggregationFrame {
        self.frame(
            AggregationPayload::Snapshot(self.aggregator.snapshot()),
            now,
        )
    }
    fn frame(&self, payload: AggregationPayload, now: Duration) -> AggregationFrame {
        AggregationFrame {
            published_unix_ns: crate::unix_ns(),
            emitted_at: now,
            payload,
        }
    }
    /// Maximum sleep until state maintenance or an earlier condition deadline.
    pub fn wait(&self, now: Duration) -> Duration {
        self.aggregator
            .next_deadline()
            .map_or(ms(self.config.maintenance_ms.unwrap()), |d| {
                d.saturating_sub(now)
                    .min(ms(self.config.maintenance_ms.unwrap()))
            })
    }
}
pub(crate) fn ms(value: u32) -> Duration {
    Duration::from_millis(value.into())
}
pub(crate) fn detail(error: &impl std::fmt::Display) -> Text {
    let s = error.to_string();
    let mut end = s.len().min(192);
    while !s.is_char_boundary(end) {
        end -= 1
    }
    Text::new(&s[..end]).expect("bounded UTF-8")
}
