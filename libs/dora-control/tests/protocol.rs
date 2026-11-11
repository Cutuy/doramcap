//! Real WebSocket frames exercise reuse, correlation, cancellation and uncertain ownership.
use dora_control::{
    DoraCoordinatorClient, Error,
    protocol::{
        self, DataflowId, SessionId,
        cli_to_coordinator::ControlRequest,
        common::{NodeError, NodeErrorCause, NodeExitStatus},
        coordinator_to_cli::{
            ControlRequestReply, DataflowIdAndName, DataflowList, DataflowListEntry,
            DataflowResult, DataflowStatus,
        },
        ws_protocol::{WsRequest, WsResponse},
    },
};
use futures_util::{SinkExt, StreamExt};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot,
    time::Instant,
};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

type Peer = WebSocketStream<TcpStream>;
const TEST_BUDGET: Duration = Duration::from_secs(5);
const SILENT_PEER_BUDGET: Duration = Duration::from_millis(100);
const RUN_NAME: &str = "owned-experiment";
const SYSTEM_NODE: &str = "system-recorder";
const NODE_STOP_GRACE: Duration = Duration::from_secs(2);
fn deadline() -> Instant {
    Instant::now() + TEST_BUDGET
}
async fn listener() -> (TcpListener, SocketAddr) {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    (listener, address)
}
async fn accept(listener: &TcpListener) -> Peer {
    tokio_tungstenite::accept_async(listener.accept().await.unwrap().0)
        .await
        .unwrap()
}
async fn receive(peer: &mut Peer) -> (WsRequest, ControlRequest) {
    let Message::Text(text) = peer.next().await.unwrap().unwrap() else {
        panic!("expected text request");
    };
    let request: WsRequest = serde_json::from_str(&text).unwrap();
    assert_eq!(request.method, "control");
    let control = serde_json::from_value(request.params.clone()).unwrap();
    (request, control)
}
async fn reply(peer: &mut Peer, request: WsRequest, reply: ControlRequestReply) {
    let response = WsResponse::ok(request.id, serde_json::to_value(reply).unwrap());
    peer.send(Message::Text(
        serde_json::to_string(&response).unwrap().into(),
    ))
    .await
    .unwrap();
}
async fn hello(peer: &mut Peer) {
    let (request, control) = receive(peer).await;
    assert!(matches!(control, ControlRequest::Hello { .. }));
    reply(
        peer,
        request,
        ControlRequestReply::HelloOk {
            dora_version: protocol::current_crate_version(),
        },
    )
    .await;
}
fn start() -> ControlRequest {
    ControlRequest::Start {
        build_id: None,
        session_id: SessionId::generate(),
        dataflow: serde_json::from_value(serde_json::json!({"nodes": []})).unwrap(),
        name: Some(RUN_NAME.into()),
        local_working_dir: Some("/runtime".into()),
        uv: false,
        write_events_to: None,
    }
}
async fn spawn_graph(peer: &mut Peer, uuid: DataflowId) {
    let (request, control) = receive(peer).await;
    assert!(matches!(control, ControlRequest::Start { name: Some(name), .. } if name == RUN_NAME));
    reply(
        peer,
        request,
        ControlRequestReply::DataflowStartTriggered { uuid },
    )
    .await;
    let (request, control) = receive(peer).await;
    assert!(matches!(control, ControlRequest::WaitForSpawn { dataflow_id } if dataflow_id == uuid));
    reply(peer, request, ControlRequestReply::DataflowSpawned { uuid }).await;
}
fn entry(name: &str, uuid: DataflowId) -> DataflowListEntry {
    DataflowListEntry {
        id: DataflowIdAndName {
            name: Some(name.into()),
            uuid,
        },
        status: DataflowStatus::Running,
    }
}
async fn stop_graph(peer: &mut Peer, uuid: DataflowId) {
    let (request, control) = receive(peer).await;
    assert!(
        matches!(control, ControlRequest::Stop { dataflow_uuid, grace_duration: None, force: true } if dataflow_uuid == uuid)
    );
    reply(
        peer,
        request,
        ControlRequestReply::DataflowStopped {
            uuid,
            result: DataflowResult::ok_empty(uuid, protocol::uhlc::HLC::default().new_timestamp()),
        },
    )
    .await;
    let (request, control) = receive(peer).await;
    assert!(matches!(control, ControlRequest::Destroy));
    reply(peer, request, ControlRequestReply::DestroyOk).await;
}

#[tokio::test]
async fn node_errors_do_not_skip_destroy_and_remain_reported() {
    for destroy_fails in [false, true] {
        // Arrange
        let (listener, address) = listener().await;
        let uuid = DataflowId::new_v4();
        let peer = tokio::spawn(async move {
            let mut peer = accept(&listener).await;
            hello(&mut peer).await;
            spawn_graph(&mut peer, uuid).await;
            let (request, control) = receive(&mut peer).await;
            assert!(
                matches!(control, ControlRequest::Stop { dataflow_uuid, force: true, .. } if dataflow_uuid == uuid)
            );
            let mut result =
                DataflowResult::ok_empty(uuid, protocol::uhlc::HLC::default().new_timestamp());
            result.node_results.insert(
                SYSTEM_NODE.to_owned().into(),
                Err(NodeError {
                    timestamp: result.timestamp,
                    cause: NodeErrorCause::Other {
                        stderr: "regression node error".into(),
                    },
                    exit_status: NodeExitStatus::ExitCode(1),
                }),
            );
            reply(
                &mut peer,
                request,
                ControlRequestReply::DataflowStopped { uuid, result },
            )
            .await;
            let (request, control) = receive(&mut peer).await;
            assert!(matches!(control, ControlRequest::Destroy));
            let response = if destroy_fails {
                ControlRequestReply::Error("destroy rejected".into())
            } else {
                ControlRequestReply::DestroyOk
            };
            reply(&mut peer, request, response).await;
            assert!(
                peer.next().await.is_none_or(|frame| {
                    frame.is_err() || matches!(frame, Ok(Message::Close(_)))
                })
            );
        });
        let mut client = DoraCoordinatorClient::new(address, None);
        client.start_graph(start(), deadline()).await.unwrap();
        // Act
        let result = client.shutdown(deadline()).await;
        // Assert: confirmed stop permits Destroy, and both failure causes stay visible.
        let Err(Error::Remote(message)) = result else {
            panic!("node error was lost: {result:?}");
        };
        assert!(message.contains(SYSTEM_NODE) && message.contains("regression node error"));
        if destroy_fails {
            assert!(message.contains("destroy rejected"));
        }
        assert_eq!(client.graph().unwrap().uuid, Some(uuid));
        peer.await.unwrap();
    }
}

#[tokio::test]
async fn one_connection_serves_readiness_registration_inspection_and_shutdown() {
    // Arrange
    let (listener, address) = listener().await;
    let uuid = DataflowId::new_v4();
    let peer = tokio::spawn(async move {
        let mut peer = accept(&listener).await;
        hello(&mut peer).await;
        for connected in [false, true] {
            let (request, control) = receive(&mut peer).await;
            assert!(matches!(control, ControlRequest::DaemonConnected));
            reply(
                &mut peer,
                request,
                ControlRequestReply::DaemonConnected(connected),
            )
            .await;
        }
        spawn_graph(&mut peer, uuid).await;
        let (request, control) = receive(&mut peer).await;
        assert!(
            matches!(control, ControlRequest::StopNode { dataflow_id, node_id, grace_duration: Some(grace) }
            if dataflow_id == uuid && node_id.as_ref() == SYSTEM_NODE && grace == NODE_STOP_GRACE)
        );
        reply(
            &mut peer,
            request,
            ControlRequestReply::NodeStopped {
                dataflow_id: uuid,
                node_id: SYSTEM_NODE.to_owned().into(),
            },
        )
        .await;
        let (request, control) = receive(&mut peer).await;
        assert!(matches!(control, ControlRequest::List));
        reply(
            &mut peer,
            request,
            ControlRequestReply::DataflowList(DataflowList(vec![entry(RUN_NAME, uuid)])),
        )
        .await;
        stop_graph(&mut peer, uuid).await;
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act / Assert: each phase uses the accepted connection.
    assert!(!client.daemon_connected(deadline()).await.unwrap());
    assert!(client.daemon_connected(deadline()).await.unwrap());
    assert_eq!(client.start_graph(start(), deadline()).await.unwrap(), uuid);
    client
        .stop_node(SYSTEM_NODE.to_owned().into(), NODE_STOP_GRACE, deadline())
        .await
        .unwrap();
    client.inspect_graph(deadline()).await.unwrap();
    client.shutdown(deadline()).await.unwrap();
    assert_eq!(client.graph().unwrap().uuid, Some(uuid));
    peer.await.unwrap();
}

#[tokio::test]
async fn cancelled_start_reconciles_the_owned_graph_without_repeating_start() {
    // Arrange
    let (listener, address) = listener().await;
    let uuid = DataflowId::new_v4();
    let (accepted_tx, accepted_rx) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let mut first = accept(&listener).await;
        hello(&mut first).await;
        let (_, control) = receive(&mut first).await;
        assert!(matches!(control, ControlRequest::Start { .. }));
        accepted_tx.send(()).unwrap();
        // The server accepted Start, but the caller cancels before its reply.
        assert!(
            first
                .next()
                .await
                .is_none_or(|frame| frame.is_err() || matches!(frame, Ok(Message::Close(_))))
        );
        let mut second = accept(&listener).await;
        hello(&mut second).await;
        let (request, control) = receive(&mut second).await;
        assert!(matches!(control, ControlRequest::List));
        reply(
            &mut second,
            request,
            ControlRequestReply::DataflowList(DataflowList(vec![
                entry("another-experiment", DataflowId::new_v4()),
                entry(RUN_NAME, uuid),
            ])),
        )
        .await;
        stop_graph(&mut second, uuid).await;
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act: cancel a Start which the coordinator has already received.
    {
        let registration = client.start_graph(start(), deadline());
        tokio::pin!(registration);
        tokio::select! {
            _ = accepted_rx => (),
            result = &mut registration => panic!("Start unexpectedly completed: {result:?}"),
        }
    }
    // Assert: identity survives and a second start is rejected locally.
    assert_eq!(client.graph().unwrap().name, RUN_NAME);
    assert!(client.graph().unwrap().uuid.is_none());
    assert!(matches!(
        client.start_graph(start(), deadline()).await,
        Err(Error::Identity(_))
    ));
    // Act / Assert: cleanup reconnects and stops only the matching UUID.
    client.shutdown(deadline()).await.unwrap();
    assert_eq!(client.graph().unwrap().uuid, Some(uuid));
    peer.await.unwrap();
}

#[tokio::test]
async fn cancellation_after_uuid_reply_keeps_identity_while_waiting_for_spawn() {
    // Arrange
    let (listener, address) = listener().await;
    let uuid = DataflowId::new_v4();
    let (waiting_tx, waiting_rx) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let mut first = accept(&listener).await;
        hello(&mut first).await;
        let (request, control) = receive(&mut first).await;
        assert!(matches!(control, ControlRequest::Start { .. }));
        reply(
            &mut first,
            request,
            ControlRequestReply::DataflowStartTriggered { uuid },
        )
        .await;
        let (_, control) = receive(&mut first).await;
        assert!(
            matches!(control, ControlRequest::WaitForSpawn { dataflow_id } if dataflow_id == uuid)
        );
        waiting_tx.send(()).unwrap();
        let _ = first.next().await;
        let mut second = accept(&listener).await;
        hello(&mut second).await;
        stop_graph(&mut second, uuid).await;
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act
    {
        let registration = client.start_graph(start(), deadline());
        tokio::pin!(registration);
        tokio::select! {
            _ = waiting_rx => (),
            result = &mut registration => panic!("spawn unexpectedly completed: {result:?}"),
        }
    }
    // Assert
    assert_eq!(client.graph().unwrap().uuid, Some(uuid));
    // Act / Assert
    client.shutdown(deadline()).await.unwrap();
    peer.await.unwrap();
}

#[tokio::test]
async fn ambiguous_registration_does_not_stop_or_destroy_anything() {
    // Arrange
    let (listener, address) = listener().await;
    let peer = tokio::spawn(async move {
        let mut peer = accept(&listener).await;
        hello(&mut peer).await;
        let (request, _) = receive(&mut peer).await;
        reply(
            &mut peer,
            request,
            ControlRequestReply::Error("start outcome unknown".into()),
        )
        .await;
        let _ = peer.next().await;
        let mut peer = accept(&listener).await;
        hello(&mut peer).await;
        let (request, control) = receive(&mut peer).await;
        assert!(matches!(control, ControlRequest::List));
        reply(
            &mut peer,
            request,
            ControlRequestReply::DataflowList(DataflowList(vec![
                entry(RUN_NAME, DataflowId::new_v4()),
                entry(RUN_NAME, DataflowId::new_v4()),
            ])),
        )
        .await;
        // No Stop or Destroy follows ambiguous ownership.
        let _ = peer.next().await;
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act / Assert
    assert!(matches!(
        client.start_graph(start(), deadline()).await,
        Err(Error::Remote(_))
    ));
    assert!(matches!(
        client.shutdown(deadline()).await,
        Err(Error::Identity(_))
    ));
    assert!(client.graph().unwrap().uuid.is_none());
    peer.await.unwrap();
}

#[tokio::test]
async fn silent_stop_expires_without_losing_graph_or_sending_destroy() {
    // Arrange
    let (listener, address) = listener().await;
    let uuid = DataflowId::new_v4();
    let peer = tokio::spawn(async move {
        let mut peer = accept(&listener).await;
        hello(&mut peer).await;
        spawn_graph(&mut peer, uuid).await;
        let (_, control) = receive(&mut peer).await;
        assert!(
            matches!(control, ControlRequest::Stop { dataflow_uuid, grace_duration: None, force: true } if dataflow_uuid == uuid)
        );
        // Remain silent; deadline expiry must close this socket without Destroy.
        assert!(
            peer.next()
                .await
                .is_none_or(|frame| frame.is_err() || matches!(frame, Ok(Message::Close(_))))
        );
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    client.start_graph(start(), deadline()).await.unwrap();
    // Act
    let result = client.shutdown(Instant::now() + SILENT_PEER_BUDGET).await;
    // Assert
    assert!(matches!(result, Err(Error::Deadline)));
    assert_eq!(client.graph().unwrap().uuid, Some(uuid));
    peer.await.unwrap();
}

#[tokio::test]
async fn silent_handshake_is_deadline_bounded() {
    // Arrange
    let (listener, address) = listener().await;
    let peer = tokio::spawn(async move {
        let mut peer = accept(&listener).await;
        let (_, control) = receive(&mut peer).await;
        assert!(matches!(control, ControlRequest::Hello { .. }));
        let _ = peer.next().await;
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act
    let result = client.connect(Instant::now() + SILENT_PEER_BUDGET).await;
    // Assert
    assert!(matches!(result, Err(Error::Deadline)));
    peer.await.unwrap();
}

#[tokio::test]
async fn different_versions_are_rejected_before_control_requests() {
    let current = protocol::current_crate_version();
    let mut major = current.clone();
    major.major += 1;
    let mut minor = current.clone();
    minor.minor += 1;
    let mut patch = current.clone();
    patch.patch += 1;
    let mut prerelease = current.clone();
    prerelease.pre = "other".parse().unwrap();
    let mut build = current.clone();
    build.build = "other".parse().unwrap();
    for version in [major, minor, patch, prerelease, build] {
        // Arrange
        let (listener, address) = listener().await;
        let peer = tokio::spawn(async move {
            let mut peer = accept(&listener).await;
            let (request, control) = receive(&mut peer).await;
            assert!(matches!(control, ControlRequest::Hello { dora_version }
                if dora_version == protocol::current_crate_version()));
            reply(
                &mut peer,
                request,
                ControlRequestReply::HelloOk {
                    dora_version: version,
                },
            )
            .await;
            assert!(matches!(
                peer.next().await,
                None | Some(Err(_)) | Some(Ok(Message::Close(_)))
            ));
        });
        let mut client = DoraCoordinatorClient::new(address, None);
        // Act
        let result = client.daemon_connected(deadline()).await;
        // Assert
        assert!(
            matches!(result, Err(Error::Protocol(message)) if message.contains("version mismatch"))
        );
        peer.await.unwrap();
    }
}

#[tokio::test]
async fn reconnect_rejects_a_different_version_before_start() {
    // Arrange
    let (listener, address) = listener().await;
    let peer = tokio::spawn(async move {
        let mut first = accept(&listener).await;
        hello(&mut first).await;
        let (request, control) = receive(&mut first).await;
        assert!(matches!(control, ControlRequest::DaemonConnected));
        reply(
            &mut first,
            request,
            ControlRequestReply::DaemonConnected(true),
        )
        .await;
        drop(first);
        let mut peer = accept(&listener).await;
        let (request, control) = receive(&mut peer).await;
        assert!(matches!(control, ControlRequest::Hello { .. }));
        let mut version = protocol::current_crate_version();
        version.patch += 1;
        reply(
            &mut peer,
            request,
            ControlRequestReply::HelloOk {
                dora_version: version,
            },
        )
        .await;
        assert!(matches!(
            peer.next().await,
            None | Some(Err(_)) | Some(Ok(Message::Close(_)))
        ));
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act
    assert!(client.daemon_connected(deadline()).await.unwrap());
    client.disconnect();
    let result = client.start_graph(start(), deadline()).await;
    // Assert
    assert!(
        matches!(result, Err(Error::Protocol(message)) if message.contains("version mismatch"))
    );
    peer.await.unwrap();
}

#[tokio::test]
async fn mismatched_reply_id_is_rejected_and_socket_is_discarded() {
    // Arrange
    let (listener, address) = listener().await;
    let peer = tokio::spawn(async move {
        let mut peer = accept(&listener).await;
        hello(&mut peer).await;
        let (mut request, control) = receive(&mut peer).await;
        assert!(matches!(control, ControlRequest::DaemonConnected));
        request.id = DataflowId::new_v4();
        reply(
            &mut peer,
            request,
            ControlRequestReply::DaemonConnected(true),
        )
        .await;
        let _ = peer.next().await;
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act
    let result = client.daemon_connected(deadline()).await;
    // Assert
    assert!(matches!(result, Err(Error::Protocol(message)) if message.contains("response ID")));
    peer.await.unwrap();
}

#[tokio::test]
async fn ping_and_explicit_authentication_work_on_the_control_connection() {
    // Arrange
    let (listener, address) = listener().await;
    let peer = tokio::spawn(async move {
        let stream = listener.accept().await.unwrap().0;
        #[allow(
            clippy::result_large_err,
            reason = "tungstenite fixes the handshake callback error type"
        )]
        fn verify_headers(
            request: &tokio_tungstenite::tungstenite::handshake::server::Request,
            response: tokio_tungstenite::tungstenite::handshake::server::Response,
        ) -> std::result::Result<
            tokio_tungstenite::tungstenite::handshake::server::Response,
            tokio_tungstenite::tungstenite::handshake::server::ErrorResponse,
        > {
            assert_eq!(request.uri().path(), "/api/control");
            assert_eq!(
                request.headers()["authorization"],
                "Bearer explicit-test-token"
            );
            Ok(response)
        }
        let mut peer = tokio_tungstenite::accept_hdr_async(stream, verify_headers)
            .await
            .unwrap();
        hello(&mut peer).await;
        let (request, control) = receive(&mut peer).await;
        assert!(matches!(control, ControlRequest::DaemonConnected));
        peer.send(Message::Ping(b"control-ping".to_vec().into()))
            .await
            .unwrap();
        let Some(Ok(Message::Pong(payload))) = peer.next().await else {
            panic!("missing Pong");
        };
        assert_eq!(payload.as_ref(), b"control-ping");
        reply(
            &mut peer,
            request,
            ControlRequestReply::DaemonConnected(true),
        )
        .await;
    });
    let mut client = DoraCoordinatorClient::new(address, Some("explicit-test-token".into()));
    // Act / Assert
    assert!(client.daemon_connected(deadline()).await.unwrap());
    peer.await.unwrap();
}

#[tokio::test]
async fn disconnect_fails_the_request_then_a_later_read_can_reconnect() {
    // Arrange
    let (listener, address) = listener().await;
    let peer = tokio::spawn(async move {
        let mut first = accept(&listener).await;
        hello(&mut first).await;
        let (_, control) = receive(&mut first).await;
        assert!(matches!(control, ControlRequest::DaemonConnected));
        drop(first);
        let mut second = accept(&listener).await;
        hello(&mut second).await;
        let (request, control) = receive(&mut second).await;
        assert!(matches!(control, ControlRequest::DaemonConnected));
        reply(
            &mut second,
            request,
            ControlRequestReply::DaemonConnected(true),
        )
        .await;
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act / Assert: the failed operation is not retried internally.
    assert!(client.daemon_connected(deadline()).await.is_err());
    // Act / Assert: a subsequent caller operation reconnects explicitly.
    assert!(client.daemon_connected(deadline()).await.unwrap());
    peer.await.unwrap();
}

#[tokio::test]
async fn expired_start_deadline_has_no_attempt_or_network_side_effect() {
    // Arrange
    let mut client = DoraCoordinatorClient::new((std::net::Ipv4Addr::LOCALHOST, 0).into(), None);
    // Act
    let result = client.start_graph(start(), Instant::now()).await;
    // Assert
    assert!(matches!(result, Err(Error::Deadline)));
    assert!(client.graph().is_none());
}

#[tokio::test]
async fn unexpected_typed_reply_fails_and_discards_the_connection() {
    // Arrange
    let (listener, address) = listener().await;
    let peer = tokio::spawn(async move {
        let mut peer = accept(&listener).await;
        hello(&mut peer).await;
        let (request, _) = receive(&mut peer).await;
        reply(&mut peer, request, ControlRequestReply::DestroyOk).await;
        let _ = peer.next().await;
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act
    let result = client.daemon_connected(deadline()).await;
    // Assert
    assert!(matches!(result, Err(Error::Protocol(message)) if message.contains("DaemonConnected")));
    peer.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn readiness_sampling_waits_for_registration_on_one_connection() {
    // Arrange
    let (listener, address) = listener().await;
    let peer = tokio::spawn(async move {
        let mut peer = accept(&listener).await;
        hello(&mut peer).await;
        for registered in [false, true] {
            let (request, control) = receive(&mut peer).await;
            assert!(matches!(control, ControlRequest::DaemonConnected));
            reply(
                &mut peer,
                request,
                ControlRequestReply::DaemonConnected(registered),
            )
            .await;
        }
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act
    client.wait_for_daemon(deadline()).await.unwrap();
    // Assert: the peer observed repeated queries on its one accepted connection.
    peer.await.unwrap();
}

#[tokio::test]
async fn missing_registration_remains_unknown_and_prevents_destroy() {
    // Arrange
    let (listener, address) = listener().await;
    let peer = tokio::spawn(async move {
        let mut first = accept(&listener).await;
        hello(&mut first).await;
        let (request, _) = receive(&mut first).await;
        reply(
            &mut first,
            request,
            ControlRequestReply::Error("start reply lost".into()),
        )
        .await;
        let _ = first.next().await;
        let mut second = accept(&listener).await;
        hello(&mut second).await;
        let (request, control) = receive(&mut second).await;
        assert!(matches!(control, ControlRequest::List));
        reply(
            &mut second,
            request,
            ControlRequestReply::DataflowList(DataflowList(vec![])),
        )
        .await;
        assert!(
            second
                .next()
                .await
                .is_none_or(|frame| frame.is_err() || matches!(frame, Ok(Message::Close(_))))
        );
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act / Assert: absence from List cannot resolve an accepted or delayed Start.
    assert!(client.start_graph(start(), deadline()).await.is_err());
    assert!(matches!(
        client.shutdown(deadline()).await,
        Err(Error::Identity(_))
    ));
    assert!(client.graph().unwrap().uuid.is_none());
    peer.await.unwrap();
}

#[tokio::test]
async fn cancelled_stop_keeps_identity_and_closes_the_inflight_connection() {
    // Arrange
    let (listener, address) = listener().await;
    let uuid = DataflowId::new_v4();
    let (stopping_tx, stopping_rx) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let mut peer = accept(&listener).await;
        hello(&mut peer).await;
        spawn_graph(&mut peer, uuid).await;
        let (_, control) = receive(&mut peer).await;
        assert!(
            matches!(control, ControlRequest::Stop { dataflow_uuid, .. } if dataflow_uuid == uuid)
        );
        stopping_tx.send(()).unwrap();
        assert!(
            peer.next()
                .await
                .is_none_or(|frame| frame.is_err() || matches!(frame, Ok(Message::Close(_))))
        );
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    client.start_graph(start(), deadline()).await.unwrap();
    // Act
    {
        let shutdown = client.shutdown(deadline());
        tokio::pin!(shutdown);
        tokio::select! {
            _ = stopping_rx => (),
            result = &mut shutdown => panic!("Stop unexpectedly completed: {result:?}"),
        }
    }
    // Assert: the retained client remains usable after its waiting future is dropped.
    assert_eq!(client.graph().unwrap().uuid, Some(uuid));
    assert_eq!(
        client.reconcile_graph(deadline()).await.unwrap(),
        Some(uuid)
    );
    peer.await.unwrap();
}

#[tokio::test]
async fn node_stop_rejects_a_reply_for_another_node_and_retains_graph_identity() {
    // Arrange
    let (listener, address) = listener().await;
    let uuid = DataflowId::new_v4();
    let peer = tokio::spawn(async move {
        let mut peer = accept(&listener).await;
        hello(&mut peer).await;
        spawn_graph(&mut peer, uuid).await;
        let (request, control) = receive(&mut peer).await;
        assert!(
            matches!(control, ControlRequest::StopNode { dataflow_id, node_id, .. }
            if dataflow_id == uuid && node_id.as_ref() == SYSTEM_NODE)
        );
        reply(
            &mut peer,
            request,
            ControlRequestReply::NodeStopped {
                dataflow_id: uuid,
                node_id: "different-node".to_owned().into(),
            },
        )
        .await;
        let _ = peer.next().await;
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    client.start_graph(start(), deadline()).await.unwrap();
    // Act
    let result = client
        .stop_node(SYSTEM_NODE.to_owned().into(), NODE_STOP_GRACE, deadline())
        .await;
    // Assert
    assert!(matches!(result, Err(Error::Protocol(_))));
    assert_eq!(client.graph().unwrap().uuid, Some(uuid));
    peer.await.unwrap();
}

#[tokio::test]
async fn start_requires_a_nonempty_explicit_name_before_any_attempt() {
    // Arrange
    let mut unnamed = start();
    if let ControlRequest::Start { name, .. } = &mut unnamed {
        *name = None;
    }
    let mut empty = start();
    if let ControlRequest::Start { name, .. } = &mut empty {
        *name = Some(String::new());
    }
    for request in [ControlRequest::List, unnamed, empty] {
        let mut client =
            DoraCoordinatorClient::new((std::net::Ipv4Addr::LOCALHOST, 0).into(), None);
        // Act
        let result = client.start_graph(request, deadline()).await;
        // Assert
        assert!(matches!(result, Err(Error::Identity(_))));
        assert!(client.graph().is_none());
    }
}

#[tokio::test(start_paused = true)]
async fn readiness_deadline_expires_when_the_daemon_never_registers() {
    // Arrange
    let (listener, address) = listener().await;
    let peer = tokio::spawn(async move {
        let mut peer = accept(&listener).await;
        hello(&mut peer).await;
        while let Some(Ok(Message::Text(text))) = peer.next().await {
            let request: WsRequest = serde_json::from_str(&text).unwrap();
            assert!(matches!(
                serde_json::from_value::<ControlRequest>(request.params.clone()).unwrap(),
                ControlRequest::DaemonConnected
            ));
            reply(
                &mut peer,
                request,
                ControlRequestReply::DaemonConnected(false),
            )
            .await;
        }
    });
    let mut client = DoraCoordinatorClient::new(address, None);
    // Act
    let result = client
        .wait_for_daemon(Instant::now() + SILENT_PEER_BUDGET)
        .await;
    // Assert
    assert!(matches!(result, Err(Error::Deadline)));
    client.disconnect();
    peer.await.unwrap();
}
