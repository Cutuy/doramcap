//! Host-owned aggregation diagnostics. Inject one instance per aggregator epoch.
use std::sync::atomic::{AtomicU64, Ordering};

/// Cumulative operational counters captured from [`FaultObservability`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Oldest output records discarded when retention is full.
    pub dropped_updates: u64,
    /// Registry-capacity and observation-evidence rejections counted by ingest.
    /// Configuration, clock, unknown-ID and envelope validation errors are excluded.
    pub rejected_inputs: u64,
    /// Observation submissions outside the retained sequence window.
    pub too_old: u64,
}

/// Host-owned diagnostics injected into one aggregator epoch.
///
/// Readers can retain an `Arc` independently of aggregator ownership. Updates
/// and reads use saturating atomic counters and require no aggregator lock.
/// No thread, exporter or I/O is created; the host chooses diagnostic publication.
#[derive(Default)]
pub struct FaultObservability {
    dropped_updates: AtomicU64,
    rejected_inputs: AtomicU64,
    too_old: AtomicU64,
}
impl FaultObservability {
    /// Create zeroed diagnostics for one aggregator epoch.
    ///
    /// Wrap in `Arc`, inject into [`crate::Aggregator::new`], and retain a clone for
    /// readers. Sharing one instance across aggregators combines their counters.
    pub fn new() -> Self {
        Self::default()
    }
    /// Copy cumulative diagnostics without borrowing the aggregator.
    ///
    /// Counters saturate at `u64::MAX` and remain readable after the aggregator is
    /// dropped. Each read is atomic; concurrent events may fall between reads, so
    /// the returned fields need not represent one instant. Performs no allocation,
    /// locking or I/O. There is no reset; create a fresh instance for a new epoch.
    pub fn stats(&self) -> Stats {
        Stats {
            dropped_updates: self.dropped_updates.load(Ordering::Relaxed),
            rejected_inputs: self.rejected_inputs.load(Ordering::Relaxed),
            too_old: self.too_old.load(Ordering::Relaxed),
        }
    }
    pub(crate) fn dropped_updates(&self) {
        increment(&self.dropped_updates);
    }
    pub(crate) fn rejected_inputs(&self) {
        increment(&self.rejected_inputs);
    }
    pub(crate) fn too_old(&self) {
        increment(&self.too_old);
    }
}
fn increment(counter: &AtomicU64) {
    let _ = counter.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
        Some(n.saturating_add(1))
    });
}
