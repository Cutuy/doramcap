//! Console live bridge: relays every Dora input to one loopback WebSocket.
mod bridge;

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
use dora_node_api::Event;
use eyre::{Result, bail};
use futures_util::{SinkExt, StreamExt};
use std::{
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
    let mut args = std::env::args().skip(1);
    let port: u16 = match (args.next().as_deref(), args.next(), args.next()) {
        (Some("--port"), Some(port), None) => port.parse()?,
        _ => bail!("usage: live-bridge --port PORT"),
    };
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
    let bridge = Arc::new(Bridge::default());
    let (attached, ready) = tokio::sync::oneshot::channel();
    let (shutdown, stopped) = tokio::sync::oneshot::channel();
    let source = bridge.clone();
    std::thread::Builder::new()
        .name("bridge-dora".into())
        .spawn(move || {
            let _ = shutdown.send(receive(&source, attached));
        })?;
    let app = Router::new()
        .route("/stream", get(stream))
        .with_state(bridge);
    tokio::pin!(stopped);
    tokio::select! {
        result = ready => result?,
        result = &mut stopped => return result?,
    }
    dora_app::environment::ready()?;
    // Stop/error terminates the server too; no orphan listener after dora stop.
    tokio::select! {
        result = axum::serve(listener, app) => result?,
        result = &mut stopped => result??,
        result = tokio::signal::ctrl_c() => result?,
    }
    Ok(())
}

/// Publish each input under its input ID until Dora stops or every input closes.
fn receive(bridge: &Bridge, attached: tokio::sync::oneshot::Sender<()>) -> Result<()> {
    let (_node, mut events) = dora_app::environment::connect()?;
    let _ = attached.send(());
    while let Some(event) = events.recv() {
        match event {
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
                bridge.publish(id.as_str(), bytes, received);
            }
            Event::Stop(_) => break,
            Event::Error(error) | Event::NodeFailed { error, .. } => bail!("Dora: {error}"),
            _ => {}
        }
    }
    Ok(())
}
