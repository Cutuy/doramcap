//! Optional, owner-thread MCAP metrics channel. Stores metadata only.
use crate::writer::ActiveWrite;
use std::{cell::RefCell, io, rc::Rc};
/// Latest owner-thread sample. Ages refer to `writer.sampled_at`.
#[derive(Clone, Copy, Debug)]
pub struct McapSnapshot {
    pub writer: crate::writer::StageSnapshot,
    pub combining_bytes: usize,
    pub combining_messages: usize,
    pub combining_capacity: usize,
    pub combining_age_us: Option<u64>,
    pub managed_available: usize,
    pub managed_capacity: usize,
    pub managed_allocated_bytes: usize,
    pub managed_free_bytes: usize,
}

#[derive(Default)]
struct State {
    snapshot: Option<McapSnapshot>,
    active: Vec<ActiveWrite>,
}
/// Single publisher, moved into McapConfig. Default disables sampling entirely.
#[derive(Default)]
pub struct McapMetrics {
    shared: Option<Rc<RefCell<State>>>,
}
/// Application-owned reader. Samples survive successful writer finalization.
#[derive(Clone)]
pub struct McapMonitor {
    shared: Rc<RefCell<State>>,
}
impl McapMonitor {
    pub fn channel() -> (McapMetrics, Self) {
        let shared = Rc::new(RefCell::new(State::default()));
        (
            McapMetrics {
                shared: Some(shared.clone()),
            },
            Self { shared },
        )
    }
    /// None until construction publishes its first sample. Ages refer to sampled_at.
    pub fn snapshot(&self) -> Option<McapSnapshot> {
        self.shared.borrow().snapshot
    }
    /// Owned metadata copy; contains no payload or allocation ownership.
    pub fn active_writes(&self) -> std::vec::IntoIter<ActiveWrite> {
        self.shared.borrow().active.clone().into_iter()
    }
}
impl McapMetrics {
    pub(crate) fn reserve(&mut self, capacity: usize) -> io::Result<()> {
        if let Some(shared) = &self.shared {
            shared
                .borrow_mut()
                .active
                .try_reserve_exact(capacity)
                .map_err(|_| io::ErrorKind::OutOfMemory)?;
        }
        Ok(())
    }
    pub(crate) fn publish<I: Iterator<Item = ActiveWrite>>(
        &self,
        snapshot: impl FnOnce() -> McapSnapshot,
        active: impl FnOnce() -> I,
    ) {
        let Some(shared) = &self.shared else { return };
        let mut state = shared.borrow_mut();
        state.snapshot = Some(snapshot());
        state.active.clear();
        state.active.extend(active());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_metrics_skip_sampling_and_iteration() {
        McapMetrics::default().publish(
            || panic!("disabled snapshot"),
            || -> std::iter::Empty<ActiveWrite> { panic!("disabled active writes") },
        );
        let (_, monitor) = McapMonitor::channel();
        assert!(monitor.snapshot().is_none());
        assert_eq!(monitor.active_writes().count(), 0);
    }
}
