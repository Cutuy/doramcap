//! Cooperative lifecycle driver; backend hooks retain resource and evidence policy.
use std::{future::Future, time::Duration};

pub type PhaseResult<T = ()> = Result<T, String>;

/// Hooks retain attempted resources before awaiting side effects. Removal must
/// honor backend evidence gates even when an earlier hook fails.
#[allow(async_fn_in_trait)]
pub trait Phases {
    async fn launch(&mut self) -> PhaseResult;
    fn ready(&mut self) -> PhaseResult;
    async fn inspect(&mut self) -> PhaseResult;
    async fn quiesce(&mut self) -> PhaseResult<bool>;
    async fn finalize(&mut self, writers_stopped: bool) -> PhaseResult;
    async fn remove(&mut self) -> PhaseResult<bool>;
    async fn finish(&mut self) -> PhaseResult;
}

/// Execution, recording and cleanup evidence remain independent.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    pub execution_error: Option<String>,
    pub writers_stopped: bool,
    pub recording_finalized: bool,
    pub cleanup_verified: bool,
    pub completion_errors: Vec<String>,
}

/// Drive startup/readiness/observation until stop, then await every completion
/// hook. Dropping this future leaves admission and recovery to the backend.
pub async fn run(phases: &mut impl Phases, stop: impl Future<Output = ()>) -> Outcome {
    tokio::pin!(stop);
    let execution = async {
        Box::pin(phases.launch()).await?;
        // Yield before publishing readiness so a stop made ready by launch wins.
        tokio::task::yield_now().await;
        phases.ready()?;
        loop {
            Box::pin(phases.inspect()).await?;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        #[allow(unreachable_code)]
        Ok::<(), String>(())
    };
    let execution_error = tokio::select! {
        biased;
        _ = &mut stop => None,
        result = execution => result.err(),
    };
    let mut outcome = Outcome {
        execution_error,
        ..Default::default()
    };
    match Box::pin(phases.quiesce()).await {
        Ok(stopped) => outcome.writers_stopped = stopped,
        Err(error) => outcome.completion_errors.push(error),
    }
    match Box::pin(phases.finalize(outcome.writers_stopped)).await {
        Ok(()) => outcome.recording_finalized = outcome.writers_stopped,
        Err(error) => outcome.completion_errors.push(error),
    }
    match Box::pin(phases.remove()).await {
        Ok(verified) => outcome.cleanup_verified = verified,
        Err(error) => outcome.completion_errors.push(error),
    }
    if let Err(error) = Box::pin(phases.finish()).await {
        outcome.completion_errors.push(error);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Hooks {
        calls: Vec<&'static str>,
        fail: Option<&'static str>,
        stop: Option<tokio::sync::oneshot::Sender<()>>,
        ready: bool,
    }
    impl Hooks {
        fn call(&mut self, phase: &'static str) -> PhaseResult {
            self.calls.push(phase);
            if self.fail == Some(phase) {
                Err(format!("{phase} failed"))
            } else {
                Ok(())
            }
        }
    }
    impl Phases for Hooks {
        async fn launch(&mut self) -> PhaseResult {
            self.call("launch")?;
            if let Some(stop) = self.stop.take() {
                stop.send(()).unwrap();
                std::future::pending::<()>().await;
            }
            Ok(())
        }
        fn ready(&mut self) -> PhaseResult {
            self.call("ready")?;
            self.ready = true;
            Ok(())
        }
        async fn inspect(&mut self) -> PhaseResult {
            self.call("inspect")?;
            Err("observation ended".into())
        }
        async fn quiesce(&mut self) -> PhaseResult<bool> {
            self.call("quiesce")?;
            Ok(true)
        }
        async fn finalize(&mut self, stopped: bool) -> PhaseResult {
            self.call("finalize")?;
            if stopped {
                Ok(())
            } else {
                Err("writers active".into())
            }
        }
        async fn remove(&mut self) -> PhaseResult<bool> {
            self.call("remove")?;
            Ok(true)
        }
        async fn finish(&mut self) -> PhaseResult {
            self.call("finish")
        }
    }
    fn hooks(fail: Option<&'static str>) -> Hooks {
        Hooks {
            calls: vec![],
            fail,
            stop: None,
            ready: false,
        }
    }
    #[tokio::test]
    async fn phase_order_and_independent_outcomes() {
        // Arrange
        let mut hooks = hooks(None);
        // Act
        let result = run(&mut hooks, std::future::pending()).await;
        // Assert
        assert_eq!(
            hooks.calls,
            [
                "launch", "ready", "inspect", "quiesce", "finalize", "remove", "finish"
            ]
        );
        assert_eq!(result.execution_error.as_deref(), Some("observation ended"));
        assert!(result.writers_stopped && result.recording_finalized && result.cleanup_verified);
    }
    #[tokio::test]
    async fn failures_preserve_completion_and_suppress_readiness() {
        for phase in ["launch", "ready", "quiesce", "finalize", "remove", "finish"] {
            // Arrange
            let mut hooks = hooks(Some(phase));
            // Act
            let result = run(&mut hooks, std::future::pending()).await;
            // Assert
            assert_eq!(hooks.calls.last(), Some(&"finish"));
            assert!(hooks.calls.contains(&"remove"));
            assert_eq!(hooks.ready, !["launch", "ready"].contains(&phase));
            if phase == "quiesce" {
                assert!(!result.writers_stopped && !result.recording_finalized);
            }
            if phase == "finalize" {
                assert!(
                    result.writers_stopped
                        && !result.recording_finalized
                        && result.cleanup_verified
                );
            }
            if phase == "remove" {
                assert!(!result.cleanup_verified);
            }
        }
    }
    #[tokio::test]
    async fn cancellation_during_launch_awaits_cleanup_without_ready() {
        // Arrange
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let mut hooks = hooks(None);
        hooks.stop = Some(stop);
        // Act
        let result = run(&mut hooks, async {
            stopped.await.unwrap();
        })
        .await;
        // Assert
        assert_eq!(
            hooks.calls,
            ["launch", "quiesce", "finalize", "remove", "finish"]
        );
        assert!(!hooks.ready);
        assert!(result.execution_error.is_none() && result.cleanup_verified);
    }
    #[tokio::test]
    async fn already_cancelled_never_launches_or_notifies() {
        // Arrange
        let mut hooks = hooks(None);
        // Act
        let result = run(&mut hooks, async {}).await;
        // Assert
        assert_eq!(hooks.calls, ["quiesce", "finalize", "remove", "finish"]);
        assert!(!hooks.ready && result.cleanup_verified);
    }
}
