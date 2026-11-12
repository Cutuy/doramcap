//! Shared run sequence: each form owns start, stop and removal behind a typed handle.
use crate::{
    launch::{CleanupRecord, RemovalPermit, StopRecord},
    recording::{FinalizeRecordings, Recording},
};
use orchestrator::LocalExecutor;
use std::future::Future;

/// One form's apps for one run, from first start attempt until removal.
///
/// The handle keeps every resource a start attempt created, so `stop` and `remove`
/// release them even after a failed or cancelled start.
#[allow(async_fn_in_trait)]
pub trait Running {
    /// Form-specific evidence handed back by removal.
    type Removed;
    /// Start every app group in order. Cancellation leaves attempted resources in the handle.
    async fn start(&mut self) -> Result<(), String>;
    /// Publish readiness after a successful start; an error ends execution and starts teardown.
    fn ready(&mut self) -> Result<(), String>;
    /// Resolve with a reason when the form can no longer continue the run.
    async fn ended(&mut self) -> String;
    /// Stop every app in reverse group order and secure form-owned evidence.
    async fn stop(&mut self) -> StopRecord;
    /// Release owned resources, discarding only what `permit` allows.
    async fn remove(self, permit: &RemovalPermit) -> (CleanupRecord, Self::Removed);
}

/// Shared results of one run; form evidence returns separately.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    /// Start, readiness or observation failure; `None` when the stop request ended the run.
    pub execution_error: Option<String>,
    pub stopped: StopRecord,
    pub permit: RemovalPermit,
    pub cleanup: CleanupRecord,
    /// Shared-phase failures, such as recordings that failed their checks.
    pub diagnostics: Vec<String>,
}

/// Start, observe until `stop` or the form ends the run, stop, check recordings, remove.
///
/// `executor` must register [`FinalizeRecordings`]. Stop and removal always run,
/// including after a failed or cancelled start. Removal is permitted only when
/// writers stopped and form evidence is durable.
pub async fn run<R: Running>(
    mut running: R,
    executor: &LocalExecutor,
    recordings: Vec<Recording>,
    stop: impl Future<Output = ()>,
) -> (Outcome, R::Removed) {
    let execution = async {
        running.start().await?;
        // Yield before publishing readiness so a stop made ready by start wins.
        tokio::task::yield_now().await;
        running.ready()?;
        Err::<(), String>(running.ended().await)
    };
    let execution_error = tokio::select! {
        biased;
        _ = stop => None,
        result = execution => result.err(),
    };
    let stopped = running.stop().await;
    let mut diagnostics = Vec::new();
    let recording_finalized = if stopped.writers_stopped {
        match executor.run::<FinalizeRecordings>(recordings).await {
            Ok(_) => true,
            Err(error) => {
                diagnostics.push(format!("finalize recording: {error}"));
                false
            }
        }
    } else {
        diagnostics.push("finalize recording: recording writers not quiescent".into());
        false
    };
    let permit = RemovalPermit {
        remove: stopped.writers_stopped && stopped.evidence_durable,
        recording_finalized,
    };
    let (cleanup, removed) = running.remove(&permit).await;
    let outcome = Outcome {
        execution_error,
        stopped,
        permit,
        cleanup,
        diagnostics,
    };
    (outcome, removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    /// Scripted form: records calls; `start` may block until a stop arrives.
    #[derive(Default)]
    struct Fake {
        calls: Vec<&'static str>,
        start_error: Option<&'static str>,
        ready_error: Option<&'static str>,
        start_blocks: Option<oneshot::Sender<()>>,
        stopped: StopRecord,
        permit: Option<RemovalPermit>,
    }
    impl Running for Fake {
        type Removed = (Vec<&'static str>, Option<RemovalPermit>);
        async fn start(&mut self) -> Result<(), String> {
            self.calls.push("start");
            if let Some(started) = self.start_blocks.take() {
                started.send(()).unwrap();
                std::future::pending::<()>().await;
            }
            self.start_error.map_or(Ok(()), |error| Err(error.into()))
        }
        fn ready(&mut self) -> Result<(), String> {
            self.calls.push("ready");
            self.ready_error.map_or(Ok(()), |error| Err(error.into()))
        }
        async fn ended(&mut self) -> String {
            self.calls.push("ended");
            "observation ended".into()
        }
        async fn stop(&mut self) -> StopRecord {
            self.calls.push("stop");
            self.stopped
        }
        async fn remove(mut self, permit: &RemovalPermit) -> (CleanupRecord, Self::Removed) {
            self.calls.push("remove");
            self.permit = Some(*permit);
            (CleanupRecord { verified: true }, (self.calls, self.permit))
        }
    }
    fn executor() -> LocalExecutor {
        let mut executor = LocalExecutor::default();
        executor.register::<FinalizeRecordings>(|| FinalizeRecordings);
        executor
    }
    fn writers_stopped() -> StopRecord {
        StopRecord {
            writers_stopped: true,
            evidence_durable: true,
        }
    }

    #[tokio::test]
    async fn form_end_becomes_the_execution_error_and_teardown_follows() {
        // Arrange
        let fake = Fake {
            stopped: writers_stopped(),
            ..Default::default()
        };
        // Act
        let (outcome, (calls, permit)) =
            run(fake, &executor(), vec![], std::future::pending()).await;
        // Assert
        assert_eq!(calls, ["start", "ready", "ended", "stop", "remove"]);
        assert_eq!(
            outcome.execution_error.as_deref(),
            Some("observation ended")
        );
        assert!(permit.unwrap().remove && outcome.permit.recording_finalized);
        assert!(outcome.cleanup.verified && outcome.diagnostics.is_empty());
    }

    #[tokio::test]
    async fn failed_start_still_stops_and_removes_without_readiness() {
        // Arrange
        let fake = Fake {
            start_error: Some("launch failed"),
            stopped: writers_stopped(),
            ..Default::default()
        };
        // Act
        let (outcome, (calls, _)) = run(fake, &executor(), vec![], std::future::pending()).await;
        // Assert
        assert_eq!(calls, ["start", "stop", "remove"]);
        assert_eq!(outcome.execution_error.as_deref(), Some("launch failed"));
    }

    #[tokio::test]
    async fn readiness_failure_still_stops_and_removes() {
        // Arrange
        let fake = Fake {
            ready_error: Some("readiness receiver dropped"),
            stopped: writers_stopped(),
            ..Default::default()
        };
        // Act
        let (outcome, (calls, permit)) =
            run(fake, &executor(), vec![], std::future::pending()).await;
        // Assert
        assert_eq!(calls, ["start", "ready", "stop", "remove"]);
        assert_eq!(
            outcome.execution_error.as_deref(),
            Some("readiness receiver dropped")
        );
        assert!(permit.unwrap().remove && outcome.cleanup.verified);
    }

    #[tokio::test]
    async fn stop_before_start_never_starts_but_removes() {
        // Arrange
        let fake = Fake {
            stopped: writers_stopped(),
            ..Default::default()
        };
        // Act
        let (outcome, (calls, _)) = run(fake, &executor(), vec![], async {}).await;
        // Assert
        assert_eq!(calls, ["stop", "remove"]);
        assert!(outcome.execution_error.is_none());
    }

    #[tokio::test]
    async fn stop_during_start_cancels_it_and_suppresses_readiness() {
        // Arrange
        let (started, startup) = oneshot::channel();
        let fake = Fake {
            start_blocks: Some(started),
            stopped: writers_stopped(),
            ..Default::default()
        };
        // Act
        let (outcome, (calls, _)) = run(fake, &executor(), vec![], async {
            startup.await.unwrap();
        })
        .await;
        // Assert
        assert_eq!(calls, ["start", "stop", "remove"]);
        assert!(outcome.execution_error.is_none());
    }

    #[tokio::test]
    async fn failed_recording_check_does_not_prevent_verified_cleanup() {
        // Arrange: a directory cannot be read as an MCAP recording.
        let fake = Fake {
            stopped: writers_stopped(),
            ..Default::default()
        };
        let recordings = vec![Recording {
            stream: std::path::PathBuf::new(),
            recording: std::env::temp_dir(),
        }];
        // Act
        let (outcome, (calls, permit)) = run(fake, &executor(), recordings, async {}).await;
        // Assert
        let permit = permit.unwrap();
        assert_eq!(calls, ["stop", "remove"]);
        assert!(outcome.execution_error.is_none() && outcome.stopped.writers_stopped);
        assert!(!permit.recording_finalized && permit.remove && outcome.cleanup.verified);
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|message| message.starts_with("finalize recording:"))
        );
    }

    #[tokio::test]
    async fn removal_requires_stopped_writers_and_durable_evidence() {
        for (writers_stopped, evidence_durable) in [(false, true), (true, false), (false, false)] {
            // Arrange
            let fake = Fake {
                stopped: StopRecord {
                    writers_stopped,
                    evidence_durable,
                },
                ..Default::default()
            };
            // Act
            let (outcome, (calls, permit)) = run(fake, &executor(), vec![], async {}).await;
            // Assert
            assert_eq!(calls.last(), Some(&"remove"));
            assert!(!permit.unwrap().remove);
            assert_eq!(outcome.permit.recording_finalized, writers_stopped);
            assert_eq!(outcome.diagnostics.is_empty(), writers_stopped);
        }
    }
}
