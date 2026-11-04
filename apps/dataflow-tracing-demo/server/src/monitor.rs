use crate::{MAX_FRAME, envelope, node_id, pb};
use axum::{
    Json, Router,
    extract::{
        State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use dora_node_api::{DoraNode, Event};
use eyre::{Result, eyre};
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use step_provenance::{Engine, Limits, pb as provenance};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
const HISTORY_BYTES: usize = 8 << 20;
const HISTORY_FRAMES: usize = 4096;
#[derive(Default)]
struct Metrics {
    nodes: BTreeMap<String, serde_json::Value>,
    steps: u64,
    activity: u64,
    closed_spans: u64,
    capture_losses: u64,
    complete: u64,
    incomplete: u64,
    rejected: u64,
    records: usize,
    pending: usize,
    recent: VecDeque<provenance::MeasurementResult>,
    patches: VecDeque<provenance::StepUpdate>,
}
struct Inner {
    sequence: u64,
    bytes: usize,
    history: VecDeque<axum::body::Bytes>,
    metrics: Metrics,
}
struct Feed {
    stream_id: String,
    inner: Mutex<Inner>,
    sender: broadcast::Sender<axum::body::Bytes>,
}
impl Feed {
    fn new() -> Self {
        Self {
            stream_id: format!(
                "{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ),
            inner: Mutex::new(Inner {
                sequence: 0,
                bytes: 0,
                history: VecDeque::new(),
                metrics: Metrics::default(),
            }),
            sender: broadcast::channel(512).0,
        }
    }
    fn push(&self, mut value: pb::Envelope) {
        let mut state = self.inner.lock().unwrap();
        state.sequence += 1;
        value.sequence = state.sequence;
        value.stream_id = self.stream_id.clone();
        let bytes = axum::body::Bytes::from(value.encode_to_vec());
        state.bytes += bytes.len();
        state.history.push_back(bytes.clone());
        while state.bytes > HISTORY_BYTES || state.history.len() > HISTORY_FRAMES {
            state.bytes -= state.history.pop_front().unwrap().len();
        }
        let _ = self.sender.send(bytes);
    }
}
fn definitions(path: &std::path::Path) -> Result<provenance::Definitions> {
    let pool = prost_reflect::DescriptorPool::decode(step_provenance::DESCRIPTOR)?;
    let descriptor = pool
        .get_message_by_name("doramcap.provenance.v1.Definitions")
        .ok_or_else(|| eyre!("missing descriptor"))?;
    let message = prost_reflect::DynamicMessage::parse_text_format(
        descriptor,
        &std::fs::read_to_string(path)?,
    )?;
    Ok(message.transcode_to()?)
}
pub async fn run(args: &[String]) -> Result<()> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut port = 8090u16;
    let mut assets = base.join("../frontend/dist");
    let mut config = base.join("../configs/measurements.pbtxt");
    for pair in args.chunks(2) {
        let value = pair.get(1).ok_or_else(|| eyre!("missing option value"))?;
        match pair[0].as_str() {
            "--port" => port = value.parse()?,
            "--assets" => assets = value.into(),
            "--measurements" => config = value.into(),
            _ => return Err(eyre!("unknown monitor option")),
        }
    }
    eyre::ensure!(
        assets.join("index.html").is_file(),
        "build the provenance frontend first"
    );
    let engine = Engine::new(
        definitions(&config)?,
        Limits {
            generations_per_stream: 64,
            ..Default::default()
        },
    )?;
    let feed = Arc::new(Feed::new());
    let f = feed.clone();
    let (stopped, mut done) = tokio::sync::mpsc::channel(1);
    std::thread::Builder::new()
        .name("provenance-collector".into())
        .spawn(move || {
            let result = receive(engine, f);
            if let Err(e) = &result {
                eprintln!("collector failed: {e:?}");
            }
            let _ = stopped.blocking_send(result);
        })?;
    let app = Router::new()
        .route("/stream", get(stream))
        .route("/api/summary", get(summary))
        .with_state(feed)
        .fallback_service(ServeDir::new(assets));
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!(
        "Monitor http://{} pid={}",
        listener.local_addr()?,
        std::process::id()
    );
    let shutdown = async move {
        tokio::select! { _ = tokio::signal::ctrl_c() => Ok(()), result = done.recv() => result.unwrap_or(Ok(())) }
    };
    // A collector stop/error ends the HTTP service too.
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        let _ = stop_rx.await;
    });
    tokio::pin!(shutdown);
    tokio::select! { result = server => result?, result = &mut shutdown => { let _ = stop_tx.send(()); result?; } }
    Ok(())
}
fn receive(mut engine: Engine, feed: Arc<Feed>) -> Result<()> {
    let (_node, mut events) = DoraNode::init_from_node_id(node_id("monitor").into())?;
    println!("READY monitor pid={}", std::process::id());
    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, .. } if id.as_str() == "keepalive" => {}
            Event::Input { data, .. } => {
                if data.null_count() != 0 {
                    return Err(eyre!("null telemetry bytes"));
                }
                let bytes: &[u8] = (&data).try_into()?;
                eyre::ensure!(bytes.len() <= MAX_FRAME, "oversized telemetry frame");
                let decoded = pb::Envelope::decode(bytes)?;
                eyre::ensure!(decoded.version == 1, "unsupported envelope version");
                let mut results = Vec::new();
                {
                    let mut state = feed.inner.lock().unwrap();
                    let m = &mut state.metrics;
                    match &decoded.body {
                        Some(pb::envelope::Body::Step(update)) => {
                            match engine.ingest(update.clone()) {
                                Ok(r) => results = r,
                                Err(_) => {
                                    m.rejected += 1;
                                    continue;
                                }
                            }
                            m.steps += 1;
                            m.patches.push_back(update.clone());
                            if m.patches.len() > 512 {
                                m.patches.pop_front();
                            }
                            let stats = engine.stats();
                            m.records = stats.records;
                            m.pending = stats.pending;
                        }
                        Some(pb::envelope::Body::Activity(batch)) => {
                            eyre::ensure!(batch.version == 1, "unsupported activity version");
                            m.activity += 1;
                            m.closed_spans +=
                                batch.records.iter().filter(|r| r.kind == 4).count() as u64;
                            // Counters in each batch are cumulative for this runtime.
                            m.capture_losses = batch.dropped_records
                                + batch.dropped_fields
                                + batch.rejected_spans
                                + batch.unregistered_records;
                        }
                        Some(pb::envelope::Body::Status(s)) => {
                            m.nodes.insert(s.node.clone(), serde_json::json!({"pid":s.pid,"epoch":s.epoch,"activity_queue_drops":s.activity_queue_drops,"simulated_report_drops":s.simulated_report_drops}));
                        }
                        Some(pb::envelope::Body::Clock(_)) => {}
                        _ => return Err(eyre!("unexpected producer message")),
                    }
                    for r in &results {
                        if r.status() == provenance::Status::Complete {
                            m.complete += 1;
                        } else {
                            m.incomplete += 1;
                        }
                        m.recent.push_back(r.clone());
                        if m.recent.len() > 256 {
                            m.recent.pop_front();
                        }
                    }
                }
                feed.push(decoded);
                for result in results {
                    feed.push(envelope(pb::envelope::Body::Result(result)));
                }
            }
            Event::Stop(_) => break,
            Event::Error(e) => return Err(eyre!("Dora error: {e}")),
            _ => {}
        }
    }
    Ok(())
}
fn reference(r: &provenance::GenerationRef) -> serde_json::Value {
    serde_json::json!({"producer":r.producer,"epoch":r.epoch,"step":r.step,"generation":r.generation,"sequence":r.sequence.to_string()})
}
async fn summary(State(feed): State<Arc<Feed>>) -> Json<serde_json::Value> {
    let s = feed.inner.lock().unwrap();
    let m = &s.metrics;
    let selected = |p: &Option<provenance::SelectedProbe>| {
        p.as_ref().map(|p| serde_json::json!({"generation":p.generation.as_ref().map(reference),"timestamp_ns":p.timestamp_ns.map(|n|n.to_string()),"boundary":p.boundary,"clock":p.clock_domain}))
    };
    Json(
        serde_json::json!({"monitor_pid":std::process::id(), "nodes":m.nodes,"steps":m.steps,"activity_batches":m.activity,"closed_spans":m.closed_spans,
        "capture_losses":m.capture_losses,"complete":m.complete,"incomplete":m.incomplete,"rejected":m.rejected,"records":m.records,"pending":m.pending,
        "replay_bytes":s.bytes,"replay_frames":s.history.len(),"sequence":s.sequence.to_string(),
        "results":m.recent.iter().map(|r| serde_json::json!({"measurement":r.measurement,"anchor":r.anchor.as_ref().map(reference),"status":r.status,"reason":r.reason,"duration_ns":r.duration_ns.map(|n|n.to_string()),"start":selected(&r.start),"end":selected(&r.end)})).collect::<Vec<_>>(),
        "patches":m.patches.iter().map(|p|serde_json::json!({"generation":p.generation.as_ref().map(reference),"start_ns":p.start_ns.map(|n|n.to_string()),"end_ns":p.end_ns.map(|n|n.to_string()),"inputs":p.inputs.as_ref().map(|i|i.refs.iter().map(reference).collect::<Vec<_>>())})).collect::<Vec<_>>() }),
    )
}
async fn stream(ws: WebSocketUpgrade, State(feed): State<Arc<Feed>>) -> Response {
    ws.on_upgrade(move |socket| serve_socket(socket, feed))
}
async fn send(
    socket: &mut futures_util::stream::SplitSink<WebSocket, WsMessage>,
    bytes: axum::body::Bytes,
) -> bool {
    matches!(
        tokio::time::timeout(
            Duration::from_secs(2),
            socket.send(WsMessage::Binary(bytes))
        )
        .await,
        Ok(Ok(()))
    )
}
async fn serve_socket(socket: WebSocket, feed: Arc<Feed>) {
    let (history, mut rx, first) = {
        let s = feed.inner.lock().unwrap();
        let first = s.sequence.saturating_sub(s.history.len() as u64);
        (s.history.clone(), feed.sender.subscribe(), first)
    };
    let (mut out, mut input) = socket.split();
    if first > 0 {
        let gap = envelope(pb::envelope::Body::Gap(pb::Gap { envelopes: first }));
        if !send(&mut out, gap.encode_to_vec().into()).await {
            return;
        }
    }
    for bytes in history {
        if !send(&mut out, bytes).await {
            return;
        }
    }
    loop {
        tokio::select! {
            result = rx.recv() => match result {
                Ok(bytes) => if !send(&mut out, bytes).await { return; },
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    let gap = envelope(pb::envelope::Body::Gap(pb::Gap { envelopes: n }));
                    if !send(&mut out, gap.encode_to_vec().into()).await { return; }
                },
                Err(_) => return,
            },
            message = input.next() => match message { None | Some(Err(_)) | Some(Ok(WsMessage::Close(_))) => return, _ => {} }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shipped_definitions_are_valid() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../configs/measurements.pbtxt");
        let defs = definitions(&path).unwrap();
        assert_eq!(defs.measurements.len(), 4);
        Engine::new(defs, Limits::default()).unwrap();
    }
    #[test]
    fn replay_is_bounded_and_sequence_is_preserved() {
        let feed = Feed::new();
        for _ in 0..(HISTORY_FRAMES + 5) {
            feed.push(envelope(pb::envelope::Body::Gap(pb::Gap { envelopes: 1 })));
        }
        let state = feed.inner.lock().unwrap();
        assert_eq!(state.history.len(), HISTORY_FRAMES);
        let first = pb::Envelope::decode(state.history.front().unwrap().as_ref()).unwrap();
        assert_eq!(first.sequence, 6);
        assert!(state.bytes <= HISTORY_BYTES);
    }
}
