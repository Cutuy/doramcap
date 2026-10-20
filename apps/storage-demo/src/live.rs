//! Bounded activity replay and live delivery; capture writes run on a sink worker.
use activity_tracing::{BatchView, Result, Sink};
use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::broadcast;
const HISTORY_BYTES: usize = 4 * 1024 * 1024;
struct FeedInner {
    history: VecDeque<axum::body::Bytes>,
    bytes: usize,
    total_bytes: u64,
    total_batches: u64,
    tx: broadcast::Sender<axum::body::Bytes>,
}
pub(crate) struct Feed(Mutex<FeedInner>, tokio::sync::watch::Sender<bool>);
impl Feed {
    pub(crate) fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self(
            Mutex::new(FeedInner {
                history: VecDeque::new(),
                bytes: 0,
                total_bytes: 0,
                total_batches: 0,
                tx,
            }),
            tokio::sync::watch::channel(false).0,
        )
    }
    pub(crate) fn shutdown(&self) {
        self.1.send_replace(true);
    }
}
pub(crate) struct LiveSink(pub(crate) Arc<Feed>);
impl Sink for LiveSink {
    fn name(&self) -> &str {
        "live"
    }
    fn write(&mut self, b: BatchView<'_>) -> Result<()> {
        let bytes = axum::body::Bytes::copy_from_slice(b.bytes);
        let mut feed = self.0.0.lock().unwrap();
        if bytes.len() > HISTORY_BYTES {
            return Err("batch exceeds demo replay budget".into());
        }
        feed.bytes += bytes.len();
        feed.total_bytes += bytes.len() as u64;
        feed.total_batches += 1;
        feed.history.push_back(bytes.clone());
        while feed.history.len() > 512 || feed.bytes > HISTORY_BYTES {
            let old = feed.history.pop_front().unwrap();
            feed.bytes -= old.len();
        }
        let _ = feed.tx.send(bytes);
        Ok(())
    }
}
pub(crate) fn router(feed: Arc<Feed>) -> Router {
    Router::new()
        .route("/api/activity/stream", get(stream))
        .with_state(feed)
}
async fn stream(ws: WebSocketUpgrade, State(feed): State<Arc<Feed>>) -> Response {
    ws.max_message_size(1024)
        .on_upgrade(move |socket| serve_socket(socket, feed))
}
async fn send(
    socket: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    message: Message,
) -> bool {
    matches!(
        tokio::time::timeout(Duration::from_secs(2), socket.send(message)).await,
        Ok(Ok(()))
    )
}
async fn serve_socket(socket: WebSocket, feed: Arc<Feed>) {
    let (mut outgoing, mut incoming) = socket.split();
    let mut stopping = feed.1.subscribe();
    if *stopping.borrow() {
        return;
    }
    let (mut rx, history) = {
        let f = feed.0.lock().unwrap();
        (
            f.tx.subscribe(),
            f.history.iter().cloned().collect::<Vec<_>>(),
        )
    };
    for bytes in history {
        if !send(&mut outgoing, Message::Binary(bytes)).await {
            return;
        }
    }
    loop {
        tokio::select! {
            _ = stopping.changed() => return,
            batch=rx.recv()=>match batch{Ok(bytes)=>if !send(&mut outgoing,Message::Binary(bytes)).await{return;},Err(broadcast::error::RecvError::Lagged(n))=>if !send(&mut outgoing,Message::Text(serde_json::json!({"type":"gap","batches":n}).to_string().into())).await{return;},Err(_)=>return},
            input=incoming.next()=>match input{Some(Ok(Message::Close(_)))|None|Some(Err(_))=>return,_=>{}}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn write(sink: &mut LiveSink, bytes: &[u8]) -> Result<()> {
        sink.write(BatchView {
            bytes,
            sequence: 1,
            unix_ns: 0,
        })
    }
    #[test]
    fn replay_is_bounded_and_slow_readers_do_not_block_capture() {
        let feed = Arc::new(Feed::new());
        let mut rx = feed.0.lock().unwrap().tx.subscribe();
        let mut sink = LiveSink(feed.clone());
        for _ in 0..600 {
            write(&mut sink, &[1; 8192]).unwrap();
        }
        let f = feed.0.lock().unwrap();
        assert_eq!(f.history.len(), 512);
        assert_eq!(f.bytes, HISTORY_BYTES);
        assert_eq!(f.total_batches, 600);
        assert!(matches!(
            rx.try_recv(),
            Err(broadcast::error::TryRecvError::Lagged(_))
        ));
    }
    #[test]
    fn oversized_batch_does_not_mutate_history() {
        let feed = Arc::new(Feed::new());
        let mut sink = LiveSink(feed.clone());
        assert!(write(&mut sink, &vec![0; HISTORY_BYTES + 1]).is_err());
        assert_eq!(feed.0.lock().unwrap().total_batches, 0);
    }
}
