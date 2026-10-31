use super::*;
use std::cell::Cell;

struct Dropped<'a>(&'a Cell<bool>);
impl Drop for Dropped<'_> {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

#[tokio::test]
async fn already_stopped_never_starts_execution() {
    // Arrange
    let started = Cell::new(false);
    let execution = async {
        started.set(true);
        Ok(())
    };
    // Act
    let result = until_stop(async {}, std::future::pending(), execution).await;
    // Assert
    assert!(result.is_none());
    assert!(!started.get());
}

#[tokio::test]
async fn stop_during_startup_drops_the_launch_and_suppresses_readiness() {
    // Arrange
    let dropped = Cell::new(false);
    let (started, startup) = oneshot::channel();
    let mut signals = RunSignals::default();
    let (ready, mut readiness) = oneshot::channel();
    signals.ready = Some(ready);
    let execution = async {
        let _launch = Dropped(&dropped);
        started.send(()).unwrap();
        std::future::pending::<()>().await;
        signals.notify_ready(Vec::new());
        Ok(())
    };
    // Act
    let result = until_stop(
        async {
            startup.await.unwrap();
        },
        std::future::pending(),
        execution,
    )
    .await;
    // Assert
    assert!(result.is_none());
    assert!(dropped.get());
    assert!(matches!(
        readiness.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn execution_and_event_failures_preserve_their_cause() {
    // Arrange / Act: execution failures reach the report.
    let execution = async { Err(orchestrator::Error::message("launch failed")) };
    let result = until_stop(std::future::pending(), std::future::pending(), execution).await;
    // Assert
    assert_eq!(result.as_deref(), Some("launch failed"));

    // Arrange / Act: an event failure interrupts a pending launch.
    let dropped = Cell::new(false);
    let (started, startup) = oneshot::channel();
    let execution = async {
        let _launch = Dropped(&dropped);
        started.send(()).unwrap();
        std::future::pending().await
    };
    let result = until_stop(
        std::future::pending(),
        async {
            startup.await.unwrap();
            "events failed".into()
        },
        execution,
    )
    .await;
    // Assert
    assert_eq!(result.as_deref(), Some("events failed"));
    assert!(dropped.get());
}
