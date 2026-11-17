//! Idempotent repository reset shared by native Build and Run.
use crate::process::HelperProcess;
use compose_apps::{compose::adapter::repository_scope, shared_dora::owner};
use docker_engine::Engine;
use std::{path::Path, time::Duration};
use tokio::time::Instant;
use workload_lifecycle::launch::RunPlan;

/// Settle prior writers, discard stale overlays and remove only owned resources under admission.
/// Resource operations share a stop deadline. Filesystem discard settles under admission even if
/// it overruns; an elapsed deadline then fails launch. Partial errors retain ownership; no write-back.
pub(crate) async fn reset_project(
    engine: &Engine,
    run: &RunPlan,
) -> std::result::Result<(), String> {
    let repository = Path::new(&run.repository_root);
    let deadline = Instant::now() + Duration::from_secs(run.stop_seconds);
    let helper =
        HelperProcess::recover_repository(repository, Duration::from_secs(run.stop_seconds))?;
    let helper_stopped = match &helper {
        Some(helper) => within(deadline, "stop previous helper", helper.quiesce()).await,
        None => Ok(()),
    };
    let scope = repository_scope(repository);
    let scope_text = scope
        .to_str()
        .ok_or("Holoscan controller scope must be UTF-8")?;
    let reservation = owner(repository);
    let resources = engine.resources(scope_text, &reservation);
    let containers_stopped = within(deadline, "stop previous containers", async {
        resources
            .quiesce(deadline.saturating_duration_since(Instant::now()).as_secs())
            .await
            .map_err(|error| error.to_string())
    })
    .await;
    helper_stopped?;
    let stopped = containers_stopped?;
    let last =
        compose_apps::system_apps::removal_owner(&stopped).map_err(|error| error.to_string())?;
    // Reset discards interrupted work only after all helper/container writers are verified stopped.
    discard_previous(&scope.join("overlays"), deadline).await?;
    within(deadline, "remove previous containers", async {
        resources
            .remove(last)
            .await
            .map_err(|error| error.to_string())
    })
    .await?;
    if let Some(helper) = helper {
        within(deadline, "remove previous helper", helper.remove()).await?;
    }
    Ok(())
}

// Tokio filesystem deletion can outlive a dropped future; settle it before admission can be released.
async fn discard_previous(workspace: &Path, deadline: Instant) -> Result<(), String> {
    #[cfg(test)]
    if let Some((entered, release)) = DISCARD_GATE.with(|gate| gate.borrow_mut().take()) {
        let _ = entered.send(deadline);
        let _ = release.await;
    }
    let result = host_filesystem::overlay::discard(workspace).await;
    let error = result
        .err()
        .map(|error| format!("discard previous overlays: {error}"));
    if Instant::now() >= deadline {
        let elapsed =
            "discard previous overlays: stop deadline elapsed after filesystem settlement";
        return Err(error.map_or_else(|| elapsed.into(), |error| format!("{error}\n{elapsed}")));
    }
    error.map_or(Ok(()), Err)
}

#[cfg(test)]
thread_local! {
    static DISCARD_GATE: std::cell::RefCell<Option<(tokio::sync::oneshot::Sender<Instant>, tokio::sync::oneshot::Receiver<()>)>> = const { std::cell::RefCell::new(None) };
}

async fn within<T>(
    deadline: Instant,
    operation: &str,
    future: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    tokio::time::timeout_at(deadline, future)
        .await
        .map_err(|_| format!("{operation}: deadline exceeded"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::DirBuilderExt;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        sync::oneshot,
    };
    use workload_lifecycle::admission::{Admission, Refusal};

    #[tokio::test]
    async fn expired_cleanup_retains_admission_until_shared_workspace_is_settled() {
        // Arrange: a controlled unsettled discard has real disposable filesystem contents.
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("holoscan-reset-{}-{unique}", std::process::id()));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .unwrap();
        let repository = root.join("repository");
        std::fs::create_dir(&repository).unwrap();
        let scope = repository_scope(&repository);
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&scope)
            .unwrap();
        let workspace = scope.join("overlays");
        for name in ["upper", "work", "merged"] {
            std::fs::create_dir_all(workspace.join("0").join(name)).unwrap();
        }
        std::fs::write(workspace.join("0/upper/output"), b"interrupted output").unwrap();
        let socket = root.join("docker.sock");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let engine = Engine::new(socket.to_str().unwrap(), 1).unwrap();
        let (server_done, mut done) = oneshot::channel();
        let server = tokio::spawn(async move {
            loop {
                let accepted = tokio::select! { _ = &mut done => break, accepted = listener.accept() => accepted.unwrap() };
                let mut connection = accepted.0;
                let mut request = Vec::new();
                let mut chunk = [0; 1024];
                while !request.ends_with(b"\r\n\r\n") {
                    let count = connection.read(&mut chunk).await.unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&chunk[..count]);
                }
                let request = String::from_utf8(request).unwrap();
                let (status, body) = if request.contains("/json HTTP/") {
                    (404, r#"{"message":"reservation absent"}"#)
                } else {
                    assert!(request.contains("/containers/json?"), "{request}");
                    (200, "[]")
                };
                connection.write_all(format!("HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let admission = Admission::acquire(&repository).unwrap();
        let run = RunPlan {
            repository_root: repository.to_str().unwrap().into(),
            stop_seconds: 1,
            ..Default::default()
        };
        let (entered, arrival) = oneshot::channel();
        let (release, resume) = oneshot::channel();
        DISCARD_GATE.with(|gate| *gate.borrow_mut() = Some((entered, resume)));
        let cleanup = async {
            let _admission = admission;
            reset_project(&engine, &run).await
        };
        tokio::pin!(cleanup);
        let observe = async {
            let deadline = arrival.await.unwrap();
            // The gate, not deletion speed, controls settlement; the actual deadline is now elapsed.
            tokio::time::sleep_until(deadline + Duration::from_millis(10)).await;
            assert!(matches!(
                Admission::acquire(&repository),
                Err(Refusal::Busy)
            ));
            assert!(
                !release.is_closed(),
                "deadline must not drop an unsettled discard"
            );
            assert!(workspace.join("0/upper/output").exists());
            release.send(()).unwrap();
        };
        // Act
        let (result, ()) = tokio::join!(&mut cleanup, observe);
        // Assert: no launch permission follows an overrun, and discarded output never writes back.
        assert!(
            result
                .unwrap_err()
                .contains("elapsed after filesystem settlement")
        );
        assert!(!workspace.exists());
        assert!(!repository.join("output").exists());
        let acquire = async {
            loop {
                match Admission::acquire(&repository) {
                    Ok(admission) => break admission,
                    Err(Refusal::Busy) => tokio::time::sleep(Duration::from_millis(1)).await,
                    Err(error) => panic!("{error}"),
                }
            }
        };
        drop(
            tokio::time::timeout(Duration::from_secs(1), acquire)
                .await
                .unwrap(),
        );
        let _ = server_done.send(());
        server.await.unwrap();
        std::fs::remove_dir_all(&scope).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn expired_discard_preserves_its_actual_failure() {
        // Arrange: a non-directory is rejected independently from the elapsed budget.
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "holoscan-reset-error-{}-{unique}",
            std::process::id()
        ));
        std::fs::write(&path, b"not an overlay directory").unwrap();
        // Act
        let error = discard_previous(&path, Instant::now()).await.unwrap_err();
        // Assert
        assert!(error.contains("canonical"), "{error}");
        assert!(
            error.contains("elapsed after filesystem settlement"),
            "{error}"
        );
        std::fs::remove_file(path).unwrap();
    }
}
