//! Cooperative execution and cancellation for the recorded Compose workflow.
use super::*;

/// Stop requests win ties; callers always perform teardown after this future returns.
pub(super) async fn until_stop(
    stop: impl Future<Output = ()>,
    event_failure: impl Future<Output = String>,
    execution: impl Future<Output = orchestrator::Result<()>>,
) -> Option<String> {
    tokio::select! {
        biased;
        _ = stop => None,
        result = execution => result.err().map(|error| error.to_string()),
        error = event_failure => Some(error),
    }
}

#[cfg(test)]
mod tests;
