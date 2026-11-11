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
