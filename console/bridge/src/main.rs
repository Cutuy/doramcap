//! Console live bridge: relays every Dora input to one loopback WebSocket.
mod bridge;
mod config;

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use bridge::Bridge;
use dora_node_api::{DoraNode, Event, EventStream};
use eyre::{Result, bail};
use fault_dora::Health;
use futures_util::{SinkExt, StreamExt};
use std::{
    collections::BTreeSet,
    net::Ipv4Addr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

async fn stream(ws: WebSocketUpgrade, State(bridge): State<Arc<Bridge>>) -> Response {
    ws.max_message_size(1024)
        .max_frame_size(1024)
        .on_upgrade(move |socket| serve_socket(socket, bridge))
}

async fn serve_socket(socket: WebSocket, bridge: Arc<Bridge>) {
    let (mut outgoing, mut incoming) = socket.split();
    let mut frames = bridge.subscribe();
    loop {
        tokio::select! {
            frame = frames.next() => match frame {
                Some(bytes) => if outgoing.send(Message::Binary(bytes.as_ref().clone().into())).await.is_err() { return },
                None => return,
            },
            message = incoming.next() => match message {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                _ => {}
            }
        }
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = match args.as_slice() {
        [flag, path] if flag == "--config" => config::parse(&std::fs::read_to_string(path)?)?,
        _ => bail!("usage: live-bridge --config FILE.pbtxt"),
    };
    let port = config.port as u16;
    let max_payload_bytes = config.max_payload_bytes as usize;
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
    let bridge = Arc::new(Bridge::new(max_payload_bytes));
    let (shutdown, stopped) = tokio::sync::oneshot::channel();
    let source = bridge.clone();
    std::thread::Builder::new()
        .name("bridge-dora".into())
        .spawn(move || {
            let _ = shutdown.send(receive(&source));
        })?;
    let app = Router::new()
        .route("/stream", get(stream))
        .with_state(bridge);
    tokio::pin!(stopped);
    // Stop/error terminates the server too; no orphan listener after dora stop.
    tokio::select! {
        result = axum::serve(listener, app) => result?,
        result = &mut stopped => result??,
        result = tokio::signal::ctrl_c() => result?,
    }
    Ok(())
}

/// Relay configured inputs until Dora stops or every non-timer input closes.
fn receive(bridge: &Bridge) -> Result<()> {
    let (node, events) = dora_app::environment::connect()?;
    let inputs = node
        .node_config()
        .inputs
        .keys()
        .map(ToString::to_string)
        .collect();
    let health = node
        .node_config()
        .outputs
        .contains("health")
        .then(Health::new)
        .transpose()?;
    forward(bridge, node, events, inputs, health)
}

const HEALTH_TICK: &str = "health_tick";

fn forward(
    bridge: &Bridge,
    mut node: DoraNode,
    mut events: EventStream,
    mut inputs: BTreeSet<String>,
    mut health: Option<Health>,
) -> Result<()> {
    inputs.remove(HEALTH_TICK);
    let check = health
        .as_ref()
        .map(|health| health.register("live-bridge", "relay", None))
        .transpose()?;
    if let Some(check) = &check {
        check.success()?;
    }
    let result = (|| {
        while !inputs.is_empty() {
            let Some(event) = events.recv() else { break };
            match event {
                Event::Input { id, .. } if id.as_str() == HEALTH_TICK => {
                    if let Some(health) = &mut health {
                        health.tick(&mut node)?;
                    }
                }
                Event::Input { id, data, .. } => {
                    let received = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .ok()
                        .and_then(|d| u64::try_from(d.as_nanos()).ok())
                        .unwrap_or_default();
                    if data.null_count() != 0 {
                        bail!("input {id} contains null bytes");
                    }
                    let bytes: &[u8] = (&data).try_into()?;
                    let accepted = bridge.publish(id.as_str(), bytes, received);
                    if let Some(check) = &check {
                        if accepted {
                            check.success()?;
                        } else {
                            Health::failure(
                                check,
                                &format!("input {id} exceeds the payload limit"),
                            )?;
                        }
                    }
                }
                Event::InputClosed { id } => {
                    inputs.remove(id.as_str());
                }
                Event::InputRecovered { id } if id.as_str() != HEALTH_TICK => {
                    inputs.insert(id.to_string());
                }
                Event::Stop(_) => break,
                Event::Error(error) | Event::NodeFailed { error, .. } => bail!("Dora: {error}"),
                _ => {}
            }
        }
        Ok(())
    })();
    if let Some(health) = &mut health {
        if let (Err(error), Some(check)) = (&result, &check) {
            let _ = Health::failure(check, &error.to_string());
        }
        // Preserve the relay's terminal outcome if final health publication fails.
        let _ = health.finish_to(&mut node, "health");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use dora_node_api::integration_testing::{
        IntegrationTestInput, TestingInput, TestingOutput,
        integration_testing_format::{IncomingEvent, InputData, TimedIncomingEvent},
        output_channel,
    };
    use futures_util::FutureExt;
    use prost::Message as _;

    #[test]
    fn health_timer_never_relays_or_keeps_closed_inputs_alive() -> Result<()> {
        // Arrange: after both real inputs close, a late timer and payload remain queued.
        let incoming = vec![
            IncomingEvent::Input {
                id: HEALTH_TICK.into(),
                metadata: None,
                data: None,
            },
            IncomingEvent::InputClosed { id: "first".into() },
            IncomingEvent::Input {
                id: HEALTH_TICK.into(),
                metadata: None,
                data: None,
            },
            IncomingEvent::Input {
                id: "second".into(),
                metadata: None,
                data: Some(Box::new(InputData::JsonObject {
                    data: vec![1u8, 2, 3].into(),
                    data_type: Some("UInt8".into()),
                })),
            },
            IncomingEvent::InputClosed {
                id: "second".into(),
            },
            IncomingEvent::Input {
                id: HEALTH_TICK.into(),
                metadata: None,
                data: None,
            },
            IncomingEvent::Input {
                id: "late".into(),
                metadata: None,
                data: None,
            },
        ];
        let input = IntegrationTestInput::new(
            "live-bridge".parse()?,
            incoming
                .into_iter()
                .map(|event| TimedIncomingEvent {
                    time_offset_secs: 0.0,
                    event,
                })
                .collect(),
        );
        let (output, _received) = output_channel();
        let (node, events) = DoraNode::init_testing(
            TestingInput::Input(input),
            TestingOutput::ToChannel(output),
            Default::default(),
        )?;
        let bridge = Bridge::new(1024);
        let inputs = ["first", "second", HEALTH_TICK].map(str::to_string).into();

        // Act
        forward(&bridge, node, events, inputs, None)?;

        // Assert: the remaining real stream relayed; timers and the late payload did not.
        let mut subscriber = bridge.subscribe();
        let bytes = subscriber.next().now_or_never().flatten().unwrap();
        let frame = bridge::pb::Frame::decode(bytes.as_slice())?;
        assert_eq!(frame.stream, "second");
        assert_eq!(frame.payload, [1, 2, 3]);
        assert!(subscriber.next().now_or_never().is_none());
        Ok(())
    }
}
