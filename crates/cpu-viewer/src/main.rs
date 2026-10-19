//! Dora subscriber and loopback HTTP server. Browser clients never block collection.
use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use dora_node_api::{DoraNode, Event};
use eyre::{Result, bail};
use futures_util::{SinkExt, StreamExt};

use std::{
    path::PathBuf,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;
use tower_http::services::ServeDir;
#[derive(Clone, Default)]
struct Latest {
    snapshot: Option<Arc<Vec<u8>>>,
    dictionary: Option<Arc<Vec<u8>>>,
}
type Feed = watch::Receiver<Latest>;
#[derive(Default)]
struct IngressMetrics {
    total_bytes: u64,
    total_messages: u64,
    interval_start: f64,
    interval_bytes: u64,
    interval_messages: u64,
    latency_sum_ms: f64,
    latency_count: u64,
    bytes_per_second: Option<f64>,
    messages_per_second: Option<f64>,
    mean_latency_ms: Option<f64>,
    invalid_timestamps: u64,
}
impl IngressMetrics {
    fn observe_dictionary(&mut self, bytes: usize) {
        self.total_bytes += bytes as u64;
        self.interval_bytes += bytes as u64;
    }

    fn observe(
        &mut self,
        bytes: usize,
        elapsed: f64,
        received_ns: Option<u64>,
        capture_ns: Option<u64>,
    ) -> monitor_wire::Ingress {
        self.total_bytes += bytes as u64;
        self.total_messages += 1;
        self.interval_bytes += bytes as u64;
        self.interval_messages += 1;
        let latency_ms = received_ns
            .zip(capture_ns)
            .and_then(|(received, capture)| received.checked_sub(capture))
            .map(|ns| ns as f64 / 1_000_000.0);
        if let Some(latency) = latency_ms {
            self.latency_sum_ms += latency;
            self.latency_count += 1;
        } else {
            self.invalid_timestamps += 1;
        }
        let interval = elapsed - self.interval_start;
        if interval >= 1.0 {
            self.bytes_per_second = Some(self.interval_bytes as f64 / interval);
            self.messages_per_second = Some(self.interval_messages as f64 / interval);
            self.mean_latency_ms =
                (self.latency_count > 0).then(|| self.latency_sum_ms / self.latency_count as f64);
            self.interval_start = elapsed;
            self.interval_bytes = 0;
            self.interval_messages = 0;
            self.latency_sum_ms = 0.0;
            self.latency_count = 0;
        }
        monitor_wire::Ingress {
            total_bytes: self.total_bytes,
            total_messages: self.total_messages,
            bytes_per_second: self.bytes_per_second,
            messages_per_second: self.messages_per_second,
            capture_to_receive_ms: latency_ms,
            mean_capture_to_receive_ms: self.mean_latency_ms,
            invalid_timestamps: self.invalid_timestamps,
        }
    }
}

async fn clock_sample() -> impl axum::response::IntoResponse {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        ns.to_string(),
    )
}

async fn stream(ws: WebSocketUpgrade, State(feed): State<Feed>) -> Response {
    ws.max_message_size(1024)
        .max_frame_size(1024)
        .on_upgrade(move |socket| serve_socket(socket, feed))
}
async fn serve_socket(socket: WebSocket, mut feed: Feed) {
    let (mut outgoing, mut incoming) = socket.split();
    let mut sent_snapshot: Option<Arc<Vec<u8>>> = None;
    let mut sent_dictionary: Option<Arc<Vec<u8>>> = None;
    loop {
        let latest = feed.borrow_and_update().clone();
        for (bytes, sent) in [
            (&latest.dictionary, &mut sent_dictionary),
            (&latest.snapshot, &mut sent_snapshot),
        ] {
            if let Some(bytes) = bytes {
                if sent
                    .as_ref()
                    .is_some_and(|previous| Arc::ptr_eq(previous, bytes))
                {
                    continue;
                }
                // One shared latest frame; slow browsers skip intermediate frames.
                if outgoing
                    .send(Message::Binary(bytes.as_ref().clone().into()))
                    .await
                    .is_err()
                {
                    return;
                }
                *sent = Some(bytes.clone());
            }
        }
        loop {
            tokio::select! {
                changed = feed.changed() => { if changed.is_err() { return; } break; }
                message = incoming.next() => match message {
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                    _ => {}
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut port = 8080u16;
    let mut assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../frontend/dist");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => {
                port = args
                    .next()
                    .ok_or_else(|| eyre::eyre!("--port requires a value"))?
                    .parse()?
            }
            "--assets" => {
                assets = args
                    .next()
                    .ok_or_else(|| eyre::eyre!("--assets requires a path"))?
                    .into()
            }
            "--help" | "-h" => {
                println!("cpu-viewer [--port 8080] [--assets frontend/dist]");
                return Ok(());
            }
            _ => bail!("unknown argument: {arg}"),
        }
    }
    if !assets.join("index.html").is_file() {
        bail!("missing frontend assets: run npm ci && npm run build in frontend, or set --assets");
    }
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let (sender, feed) = watch::channel(Latest::default());
    let (shutdown, stopped) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("viewer-dora".into())
        .spawn(move || {
            let result = receive(sender);
            let _ = shutdown.send(result);
        })?;
    let app = Router::new()
        .route("/stream", get(stream))
        .route("/clock", get(clock_sample))
        .with_state(feed)
        .fallback_service(ServeDir::new(assets));
    println!("CPU viewer: http://{}", listener.local_addr()?);
    // Stop/error terminates this node too; no orphan server after dora stop.
    tokio::select! {
        result = axum::serve(listener, app) => result?,
        result = stopped => result??,
        result = tokio::signal::ctrl_c() => result?,
    }
    Ok(())
}

fn receive(sender: watch::Sender<Latest>) -> Result<()> {
    let (_node, mut events) = DoraNode::init_from_env_force()?;
    let mut ingress = IngressMetrics::default();
    let origin = Instant::now();
    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, data, .. } if id.as_str() == "snapshots" => {
                // Timestamp immediately on dequeue, before binary decoding.
                let elapsed = origin.elapsed().as_secs_f64();
                let received_ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .ok()
                    .and_then(|d| u64::try_from(d.as_nanos()).ok());
                if data.null_count() != 0 {
                    bail!("snapshot contains null bytes");
                }
                let bytes: &[u8] = (&data).try_into()?;
                let mut frame = monitor_wire::decode(bytes)?;
                frame.viewer_ingress = Some(ingress.observe(
                    bytes.len(),
                    elapsed,
                    received_ns,
                    Some(frame.snapshot.capture_unix_ns),
                ));
                let mut encoded = Vec::new();
                monitor_wire::encode(&frame, &mut encoded)?;
                sender.send_modify(|latest| latest.snapshot = Some(Arc::new(encoded)));
            }
            Event::Input { id, data, .. } if id.as_str() == "dictionary" => {
                if data.null_count() != 0 {
                    bail!("dictionary contains null bytes");
                }
                let bytes: &[u8] = (&data).try_into()?;
                monitor_wire::decode_dictionary(bytes)?;
                ingress.observe_dictionary(bytes.len());
                sender.send_modify(|latest| latest.dictionary = Some(Arc::new(bytes.to_vec())));
            }
            Event::InputClosed { id } if id.as_str() == "snapshots" => break,
            Event::Stop(_) => break,
            Event::Error(error) | Event::NodeFailed { error, .. } => bail!("Dora: {error}"),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ingress_counts_original_binary_payload_and_latency() {
        let mut metrics = IngressMetrics::default();
        let first = metrics.observe(100, 0.25, Some(5_000_000), Some(2_000_000));
        assert!(first.bytes_per_second.is_none());
        assert_eq!(first.capture_to_receive_ms, Some(3.0));
        let next = metrics.observe(200, 1.0, Some(7_000_000), Some(2_000_000));
        assert_eq!(next.total_bytes, 300);
        assert_eq!(next.bytes_per_second, Some(300.0));
        assert_eq!(next.mean_capture_to_receive_ms, Some(4.0));
        let invalid = metrics.observe(50, 3.0, Some(1), Some(2));
        assert!(invalid.capture_to_receive_ms.is_none());
        assert_eq!(invalid.invalid_timestamps, 1);
    }
}
