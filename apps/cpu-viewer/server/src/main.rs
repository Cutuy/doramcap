//! Dora subscriber and HTTP server. Browser clients never block collection.
mod delivery;

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::Response,
    routing::get,
};
use dora_node_api::Event;
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
    ) -> delivery::Ingress {
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
        delivery::Ingress {
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

async fn ready(State(feed): State<Feed>) -> StatusCode {
    let latest = feed.borrow();
    if latest.snapshot.is_some() && latest.dictionary.is_some() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
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
    let mut headless = false;
    let mut port = 8080u16;
    let mut bind = std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);
    let mut assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/dist");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => headless = true,
            "--bind" => {
                bind = args
                    .next()
                    .ok_or_else(|| eyre::eyre!("--bind requires an IP address"))?
                    .parse()?
            }
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
                println!(
                    "cpu-viewer [--bind 127.0.0.1] [--port 8080] [--headless | --assets PATH]"
                );
                return Ok(());
            }
            _ => bail!("unknown argument: {arg}"),
        }
    }
    if !headless && !assets.join("index.html").is_file() {
        bail!(
            "missing frontend assets: run npm ci && npm run build in apps/cpu-viewer/frontend, or set --assets"
        );
    }
    let listener = tokio::net::TcpListener::bind((bind, port)).await?;
    let (sender, feed) = watch::channel(Latest::default());
    let (shutdown, stopped) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("viewer-dora".into())
        .spawn(move || {
            let result = receive(sender);
            let _ = shutdown.send(result);
        })?;
    let app = router(feed, (!headless).then_some(assets));
    println!("CPU feed: http://{}", listener.local_addr()?);
    // Stop/error terminates this node too; no orphan server after dora stop.
    tokio::select! {
        result = axum::serve(listener, app) => result?,
        result = stopped => result??,
        result = tokio::signal::ctrl_c() => result?,
    }
    Ok(())
}

/// Serve the bounded CPU feed, optionally adding the standalone gallery frontend.
fn router(feed: Feed, assets: Option<PathBuf>) -> Router {
    let app = Router::new()
        .route("/stream", get(stream))
        .route("/clock", get(clock_sample))
        .route("/health/ready", get(ready))
        .with_state(feed);
    match assets {
        Some(path) => app.fallback_service(ServeDir::new(path)),
        None => app,
    }
}

fn receive(sender: watch::Sender<Latest>) -> Result<()> {
    let (_node, mut events) = dora_app::environment::connect()?;
    dora_app::environment::ready()?;
    let mut ingress = IngressMetrics::default();
    let origin = Instant::now();
    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, data, .. } if matches!(id.as_str(), "snapshots" | "dictionary") => {
                if data.null_count() != 0 {
                    bail!("{} contains null bytes", id);
                }
                let bytes: &[u8] = (&data).try_into()?;
                ingest(id.as_str(), bytes, &sender, &mut ingress, origin)?;
            }
            Event::InputClosed { id } if id.as_str() == "snapshots" => break,
            Event::Stop(_) => break,
            Event::Error(error) | Event::NodeFailed { error, .. } => bail!("Dora: {error}"),
            _ => {}
        }
    }
    Ok(())
}

fn ingest(
    input: &str,
    bytes: &[u8],
    sender: &watch::Sender<Latest>,
    ingress: &mut IngressMetrics,
    origin: Instant,
) -> Result<()> {
    match input {
        "snapshots" => {
            // Timestamp on dequeue, before decoding.
            let elapsed = origin.elapsed().as_secs_f64();
            let received_ns = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|d| u64::try_from(d.as_nanos()).ok());
            let frame = telemetry_protocol::decode(bytes)?;
            let metrics = ingress.observe(
                bytes.len(),
                elapsed,
                received_ns,
                Some(frame.snapshot.capture_unix_ns),
            );
            let encoded = delivery::encode_snapshot(frame, metrics)?;
            sender.send_modify(|latest| latest.snapshot = Some(Arc::new(encoded)));
        }
        "dictionary" => {
            telemetry_protocol::decode_dictionary(bytes)?;
            ingress.observe_dictionary(bytes.len());
            sender.send_modify(|latest| latest.dictionary = Some(Arc::new(bytes.to_vec())));
        }
        _ => bail!("unknown CPU input: {input}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn headless_http_serves_feed_routes_without_frontend_files() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        // Arrange
        let (_sender, receiver) = watch::channel(Latest::default());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router(receiver, None)).await.unwrap();
        });
        // Act / Assert
        for (path, expected) in [("/health/ready", "503"), ("/clock", "200"), ("/", "404")] {
            let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
            socket
                .write_all(
                    format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                        .as_bytes(),
                )
                .await
                .unwrap();
            let mut response = Vec::new();
            socket.read_to_end(&mut response).await.unwrap();
            assert!(
                String::from_utf8(response)
                    .unwrap()
                    .starts_with(&format!("HTTP/1.1 {expected}"))
            );
        }
        server.abort();
    }

    #[test]
    fn stalled_and_disconnected_consumers_keep_latest_dictionary_and_loss_counters() {
        use prost::Message;
        // Arrange
        let (sender, receiver) = watch::channel(Latest::default());
        let mut ingress = IngressMetrics::default();
        let origin = Instant::now();
        let dictionary =
            telemetry_protocol::dictionary_packet(&telemetry_protocol::DictionaryFrame {
                version: telemetry_protocol::VERSION,
                session_id: "fixture".into(),
                revision: 1,
                overflow: 7,
                entries: Vec::<telemetry_protocol::DictionaryEntry>::new(),
                capture_unix_ns: 17,
            })
            .encode_to_vec();
        let original = include_bytes!("../../frontend/src/fixtures/snapshot-v2.pb");
        let mut frame = telemetry_protocol::decode(original).unwrap();
        // Act: the consumer does not poll while validated producer messages arrive.
        ingest("dictionary", &dictionary, &sender, &mut ingress, origin).unwrap();
        for sequence in 0..1000 {
            frame.snapshot.sequence = sequence;
            frame.snapshot.dropped_batches = 13;
            let packet = telemetry_protocol::snapshot_packet(&frame).encode_to_vec();
            ingest("snapshots", &packet, &sender, &mut ingress, origin).unwrap();
        }
        // Assert: only the latest snapshot is retained, together with its dictionary.
        let latest = receiver.borrow().clone();
        assert_eq!(latest.dictionary.as_ref().unwrap().as_slice(), dictionary);
        let decoded = telemetry_protocol::decode(latest.snapshot.as_ref().unwrap()).unwrap();
        assert_eq!(decoded.snapshot.sequence, 999);
        assert_eq!(decoded.snapshot.dropped_batches, 13);
        assert_eq!(ingress.total_messages, 1000);
        // Act: the browser disconnects; ingestion continues and rejects invalid telemetry.
        drop(receiver);
        ingest("snapshots", original, &sender, &mut ingress, origin).unwrap();
        let retained = sender.borrow().snapshot.clone();
        assert!(ingest("snapshots", b"invalid", &sender, &mut ingress, origin).is_err());
        // Assert
        assert_eq!(ingress.total_messages, 1001);
        assert!(Arc::ptr_eq(
            retained.as_ref().unwrap(),
            sender.borrow().snapshot.as_ref().unwrap()
        ));
    }

    #[tokio::test]
    async fn readiness_requires_snapshot_and_dictionary() {
        // Arrange
        let (sender, receiver) = watch::channel(Latest::default());
        // Act / Assert
        assert_eq!(
            ready(State(receiver.clone())).await,
            StatusCode::SERVICE_UNAVAILABLE
        );
        sender.send_modify(|value| value.snapshot = Some(Arc::new(vec![1])));
        assert_eq!(
            ready(State(receiver.clone())).await,
            StatusCode::SERVICE_UNAVAILABLE
        );
        sender.send_modify(|value| value.dictionary = Some(Arc::new(vec![2])));
        assert_eq!(ready(State(receiver)).await, StatusCode::OK);
    }
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
