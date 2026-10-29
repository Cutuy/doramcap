use axum::{
    Json, Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::Response,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::{broadcast, mpsc},
};
use tower_http::services::ServeDir;

type Shared = Arc<Mutex<Feed>>;
struct Feed {
    history: VecDeque<Vec<u8>>,
    bytes: usize,
    evicted: u64,
    tx: broadcast::Sender<Message>,
    command: Option<mpsc::Sender<bool>>,
    busy: bool,
    last: Value,
    reports: VecDeque<Value>,
}
impl Feed {
    fn status(&self) -> Value {
        json!({"type":"status", "ready": self.command.is_some(), "busy":self.busy,
        "events":self.history.len(), "bytes":self.bytes, "evicted":self.evicted, "last":self.last, "reports":self.reports})
    }
    fn announce(&self) {
        let _ = self
            .tx
            .send(Message::Text(self.status().to_string().into()));
    }
    fn push(&mut self, bytes: Vec<u8>) {
        self.bytes += bytes.len();
        self.history.push_back(bytes.clone());
        while self.history.len() > 512 || self.bytes > 4 * 1024 * 1024 {
            self.bytes -= self.history.pop_front().unwrap().len();
            self.evicted += 1;
        }
        let _ = self.tx.send(Message::Binary(bytes.into()));
    }
}
async fn status(State(feed): State<Shared>) -> Json<Value> {
    Json(feed.lock().unwrap().status())
}
async fn ready(State(feed): State<Shared>) -> StatusCode {
    if feed.lock().unwrap().command.is_some() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
#[derive(serde::Deserialize)]
struct Run {
    inject_failure: bool,
}
async fn run(State(feed): State<Shared>, Json(request): Json<Run>) -> (StatusCode, Json<Value>) {
    let mut feed = feed.lock().unwrap();
    if feed.busy {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error":"An experiment is already running"})),
        );
    }
    let Some(command) = &feed.command else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error":"Worker is offline"})),
        );
    };
    if command.try_send(request.inject_failure).is_err() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error":"Worker queue unavailable"})),
        );
    }
    feed.busy = true;
    feed.announce();
    (StatusCode::ACCEPTED, Json(json!({"accepted":true})))
}
async fn websocket(State(feed): State<Shared>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| client(socket, feed))
}
async fn send(socket: &mut WebSocket, message: Message) -> bool {
    matches!(
        tokio::time::timeout(Duration::from_secs(2), socket.send(message)).await,
        Ok(Ok(()))
    )
}
async fn client(mut socket: WebSocket, feed: Shared) {
    let (history, status, mut rx) = {
        let feed = feed.lock().unwrap();
        (feed.history.clone(), feed.status(), feed.tx.subscribe())
    };
    if !send(
        &mut socket,
        Message::Text(json!({"type":"reset"}).to_string().into()),
    )
    .await
    {
        return;
    }
    for bytes in history {
        if !send(&mut socket, Message::Binary(bytes.into())).await {
            return;
        }
    }
    if !send(&mut socket, Message::Text(status.to_string().into())).await {
        return;
    }
    loop {
        tokio::select! {
            event = rx.recv() => match event {
                Ok(event) => if !send(&mut socket, event).await { break; },
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    let _ = send(&mut socket, Message::Text(json!({"type":"gap", "count":count}).to_string().into())).await;
                    break;
                }
                Err(_) => break,
            },
            incoming = socket.recv() => match incoming {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                _ => {},
            }
        }
    }
}
async fn worker(
    stream: UnixStream,
    feed: Shared,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut read, mut write) = stream.into_split();
    let (tx, mut requests) = mpsc::channel::<bool>(1);
    {
        let mut feed = feed.lock().unwrap();
        feed.history.clear();
        feed.bytes = 0;
        feed.evicted = 0;
        feed.command = Some(tx);
        feed.busy = false;
        let _ = feed
            .tx
            .send(Message::Text(json!({"type":"reset"}).to_string().into()));
        feed.announce();
    }
    loop {
        tokio::select! {
            request = requests.recv() => {
                let Some(inject) = request else { break; };
                write.write_all(format!("{{\"inject_failure\":{inject}}}\n").as_bytes()).await?;
            }
            kind = read.read_u8() => {
                let kind = kind?;
                let size = read.read_u32().await? as usize;
                if size > 1024 * 1024 { return Err("worker frame exceeds 1 MiB".into()); }
                let mut bytes = vec![0; size]; read.read_exact(&mut bytes).await?;
                if kind == 1 {
                    let event = workflow_protocol::decode(&bytes)?;
                    if event.version != 1 { return Err("unknown workflow schema version".into()); }
                    feed.lock().unwrap().push(bytes);
                } else if kind == 2 {
                    let result: Value = serde_json::from_slice(&bytes)?;
                    let mut feed = feed.lock().unwrap(); feed.busy = false; feed.last = result.clone(); feed.reports.push_back(result); while feed.reports.len()>8 {feed.reports.pop_front();} feed.announce();
                } else { return Err("unknown frame kind".into()); }
            }
        }
    }
    Ok(())
}
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("orchestrator-demo PRIVATE_SOCKET PORT".into());
    }
    let bind: std::net::IpAddr = std::env::var("ORCHESTRATOR_BIND")
        .unwrap_or_else(|_| "127.0.0.1".into())
        .parse()?;
    let port: u16 = args[2].parse()?;
    let socket = PathBuf::from(&args[1]);
    let listener = UnixListener::bind(&socket)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
    let (tx, _) = broadcast::channel(128);
    let feed = Arc::new(Mutex::new(Feed {
        history: VecDeque::new(),
        bytes: 0,
        evicted: 0,
        tx,
        command: None,
        busy: false,
        last: Value::Null,
        reports: VecDeque::new(),
    }));
    let local_feed = feed.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            if let Err(error) = worker(stream, local_feed.clone()).await {
                eprintln!("worker disconnected: {error}");
            }
            let mut feed = local_feed.lock().unwrap();
            feed.command = None;
            feed.busy = false;
            feed.announce();
        }
    });
    let assets = std::env::var_os("ORCHESTRATOR_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/dist"));
    let app = Router::new()
        .route("/api/status", get(status))
        .route("/health/ready", get(ready))
        .route("/api/run", post(run))
        .route("/events", get(websocket))
        .fallback_service(ServeDir::new(assets))
        .with_state(feed);
    let tcp = tokio::net::TcpListener::bind((bind, port)).await?;
    println!("Orchestrator lab: http://{}", tcp.local_addr()?);
    axum::serve(tcp, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_bounds_count_and_bytes_independently() {
        // Arrange
        let (tx, _) = broadcast::channel(128);
        let mut feed = Feed {
            history: VecDeque::new(),
            bytes: 0,
            evicted: 0,
            tx,
            command: None,
            busy: false,
            last: Value::Null,
            reports: VecDeque::new(),
        };
        // Act
        for _ in 0..600 {
            feed.push(vec![0; 10]);
        }
        // Assert
        assert_eq!(feed.history.len(), 512);
        assert_eq!(feed.evicted, 88);
        // Act
        for _ in 0..6 {
            feed.push(vec![0; 1024 * 1024]);
        }
        // Assert
        assert_eq!(feed.history.len(), 4);
        assert_eq!(feed.bytes, 4 * 1024 * 1024);
    }
}
