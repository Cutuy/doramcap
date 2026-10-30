//! Bounded reduction of workflow invocation events.
use std::collections::VecDeque;
use workflow_protocol::{Event, event::State};

use crate::policy::MAX_PROGRESS_ROWS;

// Console summarizes the latest invocation of each top-level identity.
fn same_summary_step(left: &Event, right: &Event) -> bool {
    left.parent.is_none()
        && right.parent.is_none()
        && left.session == right.session
        && left.executor == right.executor
        && left.step == right.step
}

/// Current invocation rows and independent indicators of missing history.
#[derive(Default)]
pub struct Progress {
    rows: VecDeque<Event>,
    pub sequence: u64,
    pub sequence_gaps: u64,
    pub truncated: u64,
    pub overflow: u64,
    pub unknown_version: bool,
}

impl Progress {
    /// Incorporate an event within `policy::MAX_PROGRESS_ROWS`, reporting loss separately.
    pub fn apply(&mut self, mut event: Event) {
        if event.sequence > self.sequence {
            self.sequence_gaps = self.sequence_gaps.saturating_add(
                event
                    .sequence
                    .saturating_sub(self.sequence)
                    .saturating_sub(1),
            );
            self.sequence = event.sequence;
        }
        if event.version != workflow_protocol::VERSION
            || State::try_from(event.state).is_err()
            || event.state == State::Unspecified as i32
        {
            self.unknown_version = true;
            return;
        }
        // Enforce redaction even when an upstream observer is misconfigured.
        event.payload_json = None;
        event.error = event.error.map(|_| "step failed".to_owned());
        if let Some(row) = self.rows.iter_mut().find(|row| {
            row.session == event.session
                && row.executor == event.executor
                && row.invocation == event.invocation
        }) {
            *row = event;
            return;
        }
        if self.rows.len() == MAX_PROGRESS_ROWS {
            if let Some(index) = self
                .rows
                .iter()
                .enumerate()
                .position(|(index, row)| {
                    row.state != State::Started as i32
                        && (row.parent.is_some()
                            || same_summary_step(row, &event)
                            || self
                                .rows
                                .iter()
                                .skip(index + 1)
                                .any(|later| same_summary_step(row, later)))
                })
                .or_else(|| {
                    self.rows
                        .iter()
                        .position(|row| row.state != State::Started as i32)
                })
            {
                self.rows.remove(index);
                self.truncated = self.truncated.saturating_add(1);
            } else {
                self.overflow = self.overflow.saturating_add(1);
                return;
            }
        }
        self.rows.push_back(event);
    }

    /// Restore bounded, already-redacted terminal activity without replaying events.
    pub fn restore(&mut self, status: &crate::protocol::Status) {
        self.rows.clear();
        for event in status.steps.iter().take(MAX_PROGRESS_ROWS) {
            self.apply(event.clone());
        }
        self.sequence = status.sequence;
        self.sequence_gaps = u64::from(status.progress_incomplete);
        self.truncated = status.truncated_steps;
        self.overflow = status.overflow_steps;
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Snapshot rows in admission order without exposing private event payloads.
    pub fn rows(&self) -> Vec<Event> {
        self.rows.iter().cloned().collect()
    }
    /// Whether event versions, sequence gaps or active-row overflow leave progress incomplete.
    pub fn incomplete(&self) -> bool {
        self.unknown_version || self.sequence_gaps != 0 || self.overflow != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(id: u64, sequence: u64, state: State) -> Event {
        Event {
            version: workflow_protocol::VERSION,
            sequence,
            session: "session".into(),
            executor: "local".into(),
            invocation: id,
            step: "step".into(),
            state: state as i32,
            ..Default::default()
        }
    }
    #[test]
    fn repeated_invocations_are_bounded_and_terminal_rows_are_evicted() {
        // Arrange
        let mut progress = Progress::default();
        // Act
        for id in 1..=10_000 {
            progress.apply(event(id, id * 2 - 1, State::Started));
            progress.apply(event(id, id * 2, State::Succeeded));
        }
        // Assert
        assert_eq!(progress.rows().len(), 512);
        assert_eq!(progress.truncated, 9_488);
        assert_eq!(progress.overflow, 0);
        assert!(!progress.incomplete());
    }
    #[test]
    fn distinct_top_level_steps_survive_repeated_invocation_eviction() {
        // Arrange
        let mut progress = Progress::default();
        let mut lifecycle = event(1, 1, State::Succeeded);
        lifecycle.step = "compose.prepare".into();
        lifecycle.display_name = Some("Prepare project".into());
        progress.apply(lifecycle);
        // Act
        for id in 2..=2_001 {
            let mut check = event(id, id, State::Succeeded);
            check.step = "workload_docker::workflow::ObserveProjects".into();
            progress.apply(check);
        }
        // Assert
        assert_eq!(progress.rows().len(), 512);
        assert_eq!(progress.truncated, 1_489);
        assert_eq!(progress.rows()[0].step, "compose.prepare");
        assert_eq!(progress.rows().last().unwrap().invocation, 2_001);
        assert!(!progress.incomplete());
    }
    #[test]
    fn unique_completed_steps_fall_back_to_oldest_eviction() {
        // Arrange
        let mut progress = Progress::default();
        // Act
        for id in 1..=513 {
            let mut row = event(id, id, State::Succeeded);
            row.step = format!("step.{id}");
            row.display_name = Some("Step".into());
            progress.apply(row);
        }
        // Assert
        assert_eq!(progress.rows().len(), 512);
        assert_eq!(progress.rows()[0].invocation, 2);
        assert_eq!(progress.truncated, 1);
        assert_eq!(progress.overflow, 0);
    }

    #[test]
    fn active_overflow_unknown_versions_and_redaction_are_independent() {
        // Arrange
        let mut progress = Progress::default();
        for id in 1..=512 {
            progress.apply(event(id, id, State::Started));
        }
        // Act
        progress.apply(event(513, 513, State::Started));
        let mut unknown = event(1, 515, State::Failed);
        unknown.version = 99;
        progress.apply(unknown);
        let mut failed = event(1, 516, State::Failed);
        failed.payload_json = Some("secret".into());
        failed.error = Some("secret".into());
        progress.apply(failed);
        // Assert
        assert_eq!(progress.overflow, 1);
        assert_eq!(progress.sequence_gaps, 1);
        assert!(progress.incomplete());
        assert_eq!(progress.rows()[0].error.as_deref(), Some("step failed"));
        assert!(progress.rows()[0].payload_json.is_none());
    }
    #[test]
    fn final_event_loss_is_counted_even_without_a_later_event() {
        use orchestrator::{Observer, telemetry::TelemetryObserver};
        // Arrange
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(512);
        let observer = TelemetryObserver::new(16 * 1024, move |bytes| {
            sender
                .try_send(bytes.to_vec())
                .map_err(|error| std::io::Error::other(error.to_string()))
        });
        let mut event = orchestrator::Event {
            invocation: orchestrator::Invocation {
                session: "session".into(),
                id: 1,
                step: "step".into(),
                display_name: None,
                version: 1,
                executor: "local".into(),
                parent: None,
                dependencies: vec![],
            },
            state: orchestrator::State::Started,
            elapsed_ns: 1,
            duration_ns: 0,
            payload: None,
            error: None,
        };
        for id in 1..=512 {
            event.invocation.id = id;
            observer.observe(&event);
        }
        // Act: the terminal event is the final observation and cannot enter the queue.
        event.state = orchestrator::State::Succeeded;
        observer.observe(&event);
        // Assert: loss remains independently readable without receiving another event.
        assert_eq!(observer.dropped_events(), 1);
        let mut reducer = Progress::default();
        while let Ok(bytes) = receiver.try_recv() {
            reducer.apply(workflow_protocol::decode(&bytes).unwrap());
        }
        assert_eq!(reducer.rows().len(), 512);
        assert_eq!(observer.dropped_events(), 1);
    }

    #[test]
    fn terminal_states_and_missing_parents_are_preserved() {
        // Arrange
        let mut progress = Progress::default();
        // Act
        for (index, state) in [
            State::Succeeded,
            State::Failed,
            State::Cancelled,
            State::Panicked,
        ]
        .into_iter()
        .enumerate()
        {
            let mut row = event(index as u64 + 1, index as u64 + 1, state);
            row.parent = Some(999);
            progress.apply(row);
        }
        // Assert
        assert_eq!(
            progress
                .rows()
                .iter()
                .map(|row| row.state)
                .collect::<Vec<_>>(),
            vec![
                State::Succeeded as i32,
                State::Failed as i32,
                State::Cancelled as i32,
                State::Panicked as i32
            ]
        );
        assert!(progress.rows().iter().all(|row| row.parent == Some(999)));
    }
}
