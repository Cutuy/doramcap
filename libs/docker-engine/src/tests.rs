use super::*;
use serde_json::Value;
use tokio::io::AsyncWriteExt;

pub(crate) fn test_engine(
    root: &Path,
    responses: Vec<(&'static str, Value)>,
) -> (Engine, tokio::task::JoinHandle<()>) {
    test_engine_status(
        root,
        responses
            .into_iter()
            .map(|(path, body)| (path, 200, body))
            .collect(),
    )
}

pub(crate) fn test_engine_status(
    root: &Path,
    responses: Vec<(&'static str, u16, Value)>,
) -> (Engine, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let socket = root.join("engine.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let adapter = Engine::new(socket.to_str().unwrap(), 1).unwrap();
    let server = tokio::spawn(async move {
        for (path, status, response) in responses {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            assert!(line.contains(path), "unexpected Engine request: {line}");
            loop {
                line.clear();
                assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
            }
            let body = if status == 204 {
                String::new()
            } else {
                response.to_string()
            };
            stream.get_mut().write_all(format!(
                "HTTP/1.1 {status} Response\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",body.len()
            ).as_bytes()).await.unwrap();
        }
    });
    (adapter, server)
}

#[tokio::test]
async fn process_group_guard_terminates_its_spawned_child() {
    use std::{os::unix::process::ExitStatusExt, process::Stdio, time::Duration};
    // Arrange
    let mut command = tokio::process::Command::new("/bin/cat");
    command.stdin(Stdio::piped()).stdout(Stdio::null());

    // Act
    let (mut child, group) = host_filesystem::files::ProcessGroup::spawn(&mut command).unwrap();
    let pid = i32::try_from(child.id().unwrap()).unwrap();
    // SAFETY: getpgid only observes the process group of the spawned child.
    assert_eq!(unsafe { libc::getpgid(pid) }, pid);
    drop(group);

    // Assert
    let status = tokio::time::timeout(Duration::from_secs(2), child.wait())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.signal(), Some(libc::SIGKILL));
}

#[tokio::test]
async fn quiesce_sends_the_largest_representable_stop_timeout() {
    use serde_json::json;
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let containers = json!([{"Id":"writer","Labels":{CONTROLLER_LABEL:"scope"}}]);
    let (engine, server) = test_engine_status(
        root.path(),
        vec![
            (
                "/containers/reservation/json",
                404,
                json!({"message":"absent"}),
            ),
            ("/containers/json", 200, containers.clone()),
            ("/containers/writer/stop?t=2147483647 ", 204, json!(null)),
            ("/containers/json", 200, containers),
            (
                "/containers/writer/json",
                200,
                json!({"State":{"Pid":0,"Running":false}}),
            ),
        ],
    );

    // Act
    let stopped = engine
        .resources("scope", "reservation")
        .quiesce(i32::MAX as u64)
        .await
        .unwrap();

    // Assert
    assert_eq!(stopped.len(), 1);
    assert_eq!(stopped[0].id.as_deref(), Some("writer"));
    server.await.unwrap();
}

#[tokio::test]
async fn quiesce_rejects_unrepresentable_timeout_before_using_engine() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let (engine, server) = test_engine(root.path(), vec![]);
    let seconds = u64::from(i32::MAX as u32) + 1;

    // Act
    let error = engine
        .resources("scope", "reservation")
        .quiesce(seconds)
        .await
        .unwrap_err();

    // Assert
    assert!(
        matches!(error, Error::Invalid(message) if message.contains("Docker stop timeout") && message.contains(&seconds.to_string()))
    );
    server.await.unwrap();
}

#[tokio::test]
async fn removal_never_deletes_a_final_container_outside_its_scope() {
    use serde_json::json;
    // Arrange: the supplied final-container ID belongs to another controller.
    let root = tempfile::tempdir().unwrap();
    let (engine, server) = test_engine(
        root.path(),
        vec![
            ("/containers/json", json!([])),
            ("/containers/json", json!([])),
            ("/networks", json!([])),
            ("/networks", json!([])),
            ("/containers/json", json!([])),
            ("/containers/json", json!([])),
            ("/networks", json!([])),
        ],
    );
    // Act
    engine
        .resources("scope", "reservation")
        .remove(Some("foreign"))
        .await
        .unwrap();
    // Assert: the server accepts only inventory requests, with no foreign deletion.
    server.await.unwrap();
}

#[tokio::test]
async fn final_owned_container_is_removed_after_other_containers_and_networks() {
    use serde_json::json;
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let first = json!({"Id":"writer","Labels":{CONTROLLER_LABEL:"scope"}});
    let last = json!({"Id":"reservation","Labels":{CONTROLLER_LABEL:"scope"}});
    let stopped = json!({"State":{"Pid":0,"Running":false}});
    let (engine, server) = test_engine(
        root.path(),
        vec![
            ("/containers/json", json!([first, last])),
            ("/containers/writer/json", stopped.clone()),
            ("/containers/reservation/json", stopped),
            ("/containers/json", json!([first, last])),
            ("/containers/writer", json!({})),
            (
                "/networks",
                json!([{ "Id":"network", "Labels":{CONTROLLER_LABEL:"scope"} }]),
            ),
            ("/networks/network", json!({})),
            ("/networks", json!([])),
            ("/containers/json", json!([last])),
            ("/containers/reservation", json!({})),
            ("/containers/json", json!([])),
            ("/networks", json!([])),
        ],
    );
    // Act
    engine
        .resources("scope", "reservation")
        .remove(Some("reservation"))
        .await
        .unwrap();
    // Assert: the server requires each deletion in the ownership-preserving order.
    server.await.unwrap();
}
