//! Bounded reporter draining on an application's existing Dora publication loop.
use dora_node_api::DoraNode;
use eyre::Result;
use fault_protocol::{ReporterFrame, ReporterPayload};
pub use fault_reporter::Check;
use fault_reporter::{Definition, Publisher, Reporter, Text};
use std::time::{Duration, Instant};

const MAX_DETAIL_BYTES: usize = 192;
const MAX_BATCH_RECORDS: usize = 512;
const MAX_BATCHES_PER_TURN: usize = 4;
const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(500);

pub struct Health {
    reporter: Reporter,
    publisher: Publisher,
    snapshot: Instant,
}
impl Health {
    /// Creates a reporter with a fresh process incarnation and bounded queues.
    pub fn new() -> Result<Self> {
        let (reporter, publisher) =
            Reporter::new(fault_protocol::host::incarnation()?, Default::default())?;
        Ok(Self {
            reporter,
            publisher,
            snapshot: Instant::now(),
        })
    }
    /// Registers an app-owned condition; an optional deadline detects stalled progress.
    pub fn register(
        &self,
        app: &str,
        operation: &str,
        deadline: Option<Duration>,
    ) -> Result<Check> {
        let mut definition = Definition::new(app, "runtime", operation)?;
        definition.policy.deadline = deadline;
        Ok(self.reporter.register(definition)?)
    }
    /// Records bounded failure detail without making transport calls.
    pub fn failure(check: &Check, detail: &str) -> Result<()> {
        let mut end = detail.len().min(MAX_DETAIL_BYTES);
        while !detail.is_char_boundary(end) {
            end -= 1;
        }
        let detail = &detail[..end];
        check.failure(Text::new(if detail.is_empty() {
            "operation failed"
        } else {
            detail
        })?)?;
        Ok(())
    }
    /// Returns a reporting handle sharing this publisher's bounded storage.
    pub fn reporter(&self) -> Reporter {
        self.reporter.clone()
    }
    /// Publishes bounded batches and periodic repair snapshots, including with no subscribers.
    pub fn tick(&mut self, node: &mut DoraNode) -> Result<()> {
        self.tick_to(node, "health")
    }
    /// Drains reporter publications to an explicitly declared Dora output.
    pub fn tick_to(&mut self, node: &mut DoraNode, output: &str) -> Result<()> {
        self.publisher.tick();
        if let Some(snapshot) = self.publisher.take_reconciliation() {
            self.send(node, output, ReporterPayload::Snapshot(snapshot))?;
            self.snapshot = Instant::now();
        }
        // Bound each turn even when producers continuously replenish the queues.
        for _ in 0..MAX_BATCHES_PER_TURN {
            if !self.publisher.has_records() {
                break;
            }
            let batch = self.publisher.batch(MAX_BATCH_RECORDS)?;
            if batch.observations.is_empty() {
                break;
            }
            self.send(node, output, ReporterPayload::Batch(batch))?;
        }
        if self.snapshot.elapsed() >= SNAPSHOT_INTERVAL {
            self.send(
                node,
                output,
                ReporterPayload::Snapshot(self.publisher.snapshot()),
            )?;
            self.snapshot = Instant::now();
        }
        Ok(())
    }
    /// Closes reporter admission and publishes its final state.
    pub fn finish_to(&mut self, node: &mut DoraNode, output: &str) -> Result<()> {
        self.reporter.close();
        self.send(
            node,
            output,
            ReporterPayload::Snapshot(self.publisher.final_snapshot()?),
        )
    }
    fn send(&self, node: &mut DoraNode, output: &str, payload: ReporterPayload) -> Result<()> {
        let bytes = fault_protocol::encode_reporter(
            &ReporterFrame {
                published_unix_ns: fault_protocol::host::unix_ns(),
                payload,
            },
            Default::default(),
        )?;
        node.send_output_raw(output.into(), Default::default(), bytes.len(), |out| {
            out.copy_from_slice(&bytes)
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_utf8_errors_remain_observable_failures() {
        // Arrange
        let health = Health::new().unwrap();
        let check = health.register("test", "write", None).unwrap();
        let detail = format!("error: {}", "\u{00e9}".repeat(100));
        // Act
        Health::failure(&check, &detail).unwrap();
        let snapshot = health.publisher.snapshot();
        // Assert
        let latest = snapshot.conditions[0].latest.unwrap();
        assert_eq!(latest.outcome, fault_reporter::Outcome::Failure);
        assert!(!latest.detail.as_str().is_empty());
        assert!(detail.starts_with(latest.detail.as_str()));
    }
}
