use super::*;

#[tokio::test]
async fn cancelled_failure_wait_keeps_capture_alive_and_preserves_the_error() {
    // Arrange
    let (finish, mut finished) = watch::channel(false);
    let (fault_tx, fault) = watch::channel(None);
    let task = tokio::spawn(async move {
        finished
            .changed()
            .await
            .map_err(|error| invalid(error.to_string()))?;
        Ok(())
    });
    let capture = EventCapture {
        finish,
        fault,
        task,
    };

    // Act: abandon a pending wait, as startup cancellation does.
    let mut wait = Box::pin(capture.failure());
    assert!(futures_util::poll!(wait.as_mut()).is_pending());
    drop(wait);

    // Assert
    assert!(!capture.task.is_finished());

    // Act: deliver the error between the cancelled wait and its replacement.
    fault_tx.send(Some("event stream failed".into())).unwrap();

    // Assert
    assert_eq!(capture.failure().await, "event stream failed");
    assert!(!capture.task.is_finished());

    // Act: finish capture explicitly.
    capture.finish().await.unwrap();
}
