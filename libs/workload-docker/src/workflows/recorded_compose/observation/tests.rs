use super::*;

fn event(action: &str, attributes: &[(&str, &str)]) -> EventMessage {
    EventMessage {
        action: Some(action.into()),
        time_nano: Some(5_000_000_000),
        actor: Some(bollard::models::EventActor {
            id: Some("container".into()),
            attributes: Some(
                attributes
                    .iter()
                    .map(|(key, value)| ((*key).into(), (*value).into()))
                    .collect(),
            ),
        }),
        ..Default::default()
    }
}

#[test]
fn stop_signal_starts_a_wait_and_exit_ends_it() {
    // Arrange
    let service = ("com.docker.compose.service", "perception");
    let exited = StopProgress::Exited {
        container: "container".into(),
    };

    // Act / Assert: SIGTERM and the SIGINT Dora services use both start a wait.
    for signal in ["15", "2"] {
        let kill = event("kill", &[("signal", signal), service]);
        assert_eq!(
            stop_progress(&kill),
            Some(StopProgress::Signaled {
                container: "container".into(),
                service: "perception".into(),
                signaled_unix_ms: 5_000,
            })
        );
    }
    // Act / Assert: the force kill changes nothing; removal also ends a wait.
    assert_eq!(
        stop_progress(&event("kill", &[("signal", "9"), service])),
        None
    );
    assert_eq!(stop_progress(&event("die", &[service])), Some(exited));
    assert!(stop_progress(&event("destroy", &[])).is_some());
    // Act / Assert: unrelated events are ignored.
    assert_eq!(stop_progress(&event("start", &[service])), None);
}

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
        changes: watch::channel(()).1,
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
