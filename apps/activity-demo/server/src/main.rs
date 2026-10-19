use activity_tracing::{
    ActivityContext, BatchView, Config, Handle, Runtime, Sink, sinks::McapSink,
};
use axum::{
    Json, Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::{get, post},
};
use crossbeam_channel::{Receiver, Sender, bounded};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
type Result<T> = activity_tracing::Result<T>;
const NAMES: [&str; 4] = ["Aster", "Birch", "Cedar", "Dune"];
const HELLOS: [&str; 4] = [
    "Hi from Aster!",
    "Hi from Birch!",
    "Hi from Cedar!",
    "Hi from Dune!",
];
const HISTORY_BYTES: usize = 4 * 1024 * 1024;
struct FeedInner {
    history: VecDeque<axum::body::Bytes>,
    bytes: usize,
    total_bytes: u64,
    total_batches: u64,
    tx: broadcast::Sender<axum::body::Bytes>,
}
struct Feed(Mutex<FeedInner>);
impl Feed {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self(Mutex::new(FeedInner {
            history: VecDeque::new(),
            bytes: 0,
            total_bytes: 0,
            total_batches: 0,
            tx,
        }))
    }
}
struct LiveSink(Arc<Feed>);
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
#[derive(Clone)]
struct Envelope {
    id: u64,
    from: usize,
    reply: bool,
    context: ActivityContext,
}
enum Command {
    Greet(u64),
    Deliver(Envelope),
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
struct SleepSettings {
    mu_ms: u32,
    sigma_ms: u32,
}
impl SleepSettings {
    fn valid(self) -> bool {
        self.mu_ms <= 2000 && self.sigma_ms <= 1000
    }
    fn pack(self) -> u64 {
        (u64::from(self.mu_ms) << 32) | u64::from(self.sigma_ms)
    }
    fn unpack(value: u64) -> Self {
        Self {
            mu_ms: (value >> 32) as u32,
            sigma_ms: value as u32,
        }
    }
    fn sample(self, rng: &mut impl Rng) -> u64 {
        // Box–Muller; 1-random keeps ln's argument strictly positive.
        let z = (-2.0 * (1.0 - rng.random::<f64>()).ln()).sqrt()
            * (std::f64::consts::TAU * rng.random::<f64>()).cos();
        (f64::from(self.mu_ms) + f64::from(self.sigma_ms) * z)
            .clamp(0.0, 5000.0)
            .round() as u64
    }
}
struct Broker {
    sleep: AtomicU64,
    mailboxes: Vec<Sender<Command>>,
    next: AtomicU64,
    published: AtomicU64,
    received: AtomicU64,
    dropped: AtomicU64,
    round: AtomicU64,
}
impl Broker {
    fn kick(&self) {
        let round = self.round.fetch_add(1, Ordering::Relaxed);
        if self.mailboxes[round as usize % NAMES.len()]
            .try_send(Command::Greet(round))
            .is_err()
        {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn publish(&self, handle: &Handle, from: usize, recipient: Option<usize>) {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let topic = recipient.map_or("hello", |i| NAMES[i]);
        let span = tracing::info_span!(target:"activity::demo","publish",message.id=id,topic,from=NAMES[from],reply=recipient.is_some());
        let _e = span.enter();
        let context = handle.context(&span).expect("demo subscriber context");
        self.published.fetch_add(1, Ordering::Relaxed);
        for (to, mailbox) in self.mailboxes.iter().enumerate() {
            if recipient.map_or(to != from, |target| to == target)
                && mailbox
                    .try_send(Command::Deliver(Envelope {
                        id,
                        from,
                        reply: recipient.is_some(),
                        context,
                    }))
                    .is_err()
            {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(target:"activity::demo",phase="drop",message.id=id,to=NAMES[to]);
            }
        }
        tracing::info!(target:"activity::demo",phase="publish",message.id=id,topic,from=NAMES[from],text=if recipient.is_some(){"Hi back!"}else{HELLOS[from]});
    }
}
fn sleep_span(settings: SleepSettings, rng: &mut impl Rng) {
    let ms = settings.sample(rng);
    let span = tracing::info_span!(target:"activity::demo","sleep",duration_ms=ms,mu_ms=settings.mu_ms,sigma_ms=settings.sigma_ms);
    let _e = span.enter();
    thread::sleep(Duration::from_millis(ms));
}
fn worker(
    index: usize,
    rx: Receiver<Command>,
    broker: Arc<Broker>,
    handle: Handle,
    dispatch: tracing::Dispatch,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    let _registration = handle.register_thread(NAMES[index])?;
    let _default = tracing::dispatcher::set_default(&dispatch);
    let mut rng = rand::rng();
    while !stop.load(Ordering::Acquire) {
        let command = match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(c) => c,
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        };
        match command {
            Command::Greet(round) => {
                let span = tracing::info_span!(target:"activity::demo","greeting",task.name=NAMES[index],round);
                let _e = span.enter();
                sleep_span(
                    SleepSettings::unpack(broker.sleep.load(Ordering::Relaxed)),
                    &mut rng,
                );
                broker.publish(&handle, index, None);
            }
            Command::Deliver(message) => {
                handle.with_parent(message.context,||{
                    let span=tracing::info_span!(target:"activity::demo","receive",message.id=message.id,from=NAMES[message.from],to=NAMES[index],topic=if message.reply {NAMES[index]}else{"hello"},task.name=NAMES[index]);let _e=span.enter();
                    broker.received.fetch_add(1,Ordering::Relaxed);
                    tracing::info!(target:"activity::demo",phase="receive",message.id=message.id,from=NAMES[message.from],to=NAMES[index],text=if message.reply {"Hi back!"}else{HELLOS[message.from]});
                    sleep_span(SleepSettings::unpack(broker.sleep.load(Ordering::Relaxed)), &mut rng);
                    {let work=tracing::info_span!(target:"activity::demo","work",task.name=NAMES[index]);let _w=work.enter();thread::sleep(Duration::from_millis(45+(index as u64)*20));}
                    if !message.reply {broker.publish(&handle,index,Some(message.from));}
                })?;
            }
        }
    }
    Ok(())
}
#[derive(Clone)]
struct App {
    frequency: tokio::sync::watch::Sender<f64>,
    feed: Arc<Feed>,
    broker: Arc<Broker>,
    paused: Arc<AtomicBool>,
    handle: Handle,
    recording: Arc<str>,
}
#[derive(Serialize)]
struct Status {
    frequency_hz: f64,
    conversations: u64,
    resources: serde_json::Value,
    sleep: SleepSettings,
    workers: [&'static str; 4],
    paused: bool,
    published: u64,
    received: u64,
    mailbox_drops: u64,
    capture_drops: u64,
    field_losses: u64,
    recording: String,
}
fn process_memory() -> (Option<u64>, Option<u64>) {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return (None, None);
    };
    let bytes = |key: &str| {
        status.lines().find_map(|line| {
            let value = line
                .strip_prefix(key)?
                .split_whitespace()
                .next()?
                .parse::<u64>()
                .ok()?;
            Some(value * 1024)
        })
    };
    (bytes("VmRSS:"), bytes("VmHWM:"))
}
async fn status(State(a): State<App>) -> Json<Status> {
    let losses = a.handle.losses();
    let buffers = a.handle.buffers();
    let (rss, peak_rss) = process_memory();
    let feed = a.feed.0.lock().unwrap();
    let resources = serde_json::json!({
        "rss_bytes": rss, "peak_rss_bytes": peak_rss,
        "capture": {"used": buffers.capture_blocks.saturating_sub(buffers.capture_free), "capacity": buffers.capture_blocks, "ready": buffers.capture_ready, "reserved_bytes": buffers.capture_reserved_bytes},
        "encoded": {"used": buffers.encoded_buffers.saturating_sub(buffers.encoded_free), "capacity": buffers.encoded_buffers, "reserved_bytes": buffers.encoded_reserved_bytes},
        "replay": {"bytes": feed.bytes, "byte_capacity": HISTORY_BYTES, "batches": feed.history.len(), "batch_capacity": 512},
        "live": {"clients": feed.tx.receiver_count(), "pending_batches": feed.tx.len(), "capacity": 256, "total_bytes": feed.total_bytes, "total_batches": feed.total_batches},
        "mailboxes": a.broker.mailboxes.iter().enumerate().map(|(i, tx)| serde_json::json!({"name": NAMES[i], "used": tx.len(), "capacity": tx.capacity().unwrap_or(0)})).collect::<Vec<_>>(),
        "registered_threads": buffers.registered_threads,
    });
    drop(feed);
    Json(Status {
        frequency_hz: *a.frequency.borrow(),
        conversations: a.broker.round.load(Ordering::Relaxed),
        resources,
        sleep: SleepSettings::unpack(a.broker.sleep.load(Ordering::Relaxed)),
        workers: NAMES,
        paused: a.paused.load(Ordering::Acquire),
        published: a.broker.published.load(Ordering::Relaxed),
        received: a.broker.received.load(Ordering::Relaxed),
        mailbox_drops: a.broker.dropped.load(Ordering::Relaxed),
        capture_drops: losses.records + losses.encoded_batches + losses.unregistered,
        field_losses: losses.fields,
        recording: a.recording.to_string(),
    })
}
#[derive(Deserialize)]
struct Control {
    paused: bool,
}
async fn control(State(a): State<App>, Json(c): Json<Control>) -> Json<serde_json::Value> {
    a.paused.store(c.paused, Ordering::Release);
    Json(serde_json::json!({"paused":c.paused}))
}
async fn configure_sleep(
    State(a): State<App>,
    Json(settings): Json<SleepSettings>,
) -> std::result::Result<Json<SleepSettings>, (axum::http::StatusCode, &'static str)> {
    if !settings.valid() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "mu_ms must be 0..2000; sigma_ms must be 0..1000",
        ));
    }
    a.broker.sleep.store(settings.pack(), Ordering::Relaxed);
    Ok(Json(settings))
}
#[derive(Deserialize, Serialize)]
struct PlaygroundSettings {
    #[serde(flatten)]
    sleep: SleepSettings,
    frequency_hz: f64,
}
impl PlaygroundSettings {
    fn valid(&self) -> bool {
        self.sleep.valid()
            && self.frequency_hz.is_finite()
            && (0.1..=20.0).contains(&self.frequency_hz)
    }
}
async fn configure_playground(
    State(a): State<App>,
    Json(settings): Json<PlaygroundSettings>,
) -> std::result::Result<Json<PlaygroundSettings>, (axum::http::StatusCode, &'static str)> {
    if !settings.valid() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid sleep settings or frequency outside 0.1..20 Hz",
        ));
    }
    a.broker
        .sleep
        .store(settings.sleep.pack(), Ordering::Relaxed);
    a.frequency.send_if_modified(|frequency| {
        if *frequency == settings.frequency_hz {
            false
        } else {
            *frequency = settings.frequency_hz;
            true
        }
    });
    Ok(Json(settings))
}
async fn greet(State(a): State<App>) -> Json<serde_json::Value> {
    a.broker.kick();
    Json(serde_json::json!({"ok":true}))
}
async fn stream(ws: WebSocketUpgrade, State(a): State<App>) -> Response {
    ws.max_message_size(1024)
        .on_upgrade(move |socket| serve_socket(socket, a.feed))
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
            batch=rx.recv()=>match batch{Ok(bytes)=>if !send(&mut outgoing,Message::Binary(bytes)).await{return;},Err(broadcast::error::RecvError::Lagged(n))=>if !send(&mut outgoing,Message::Text(serde_json::json!({"type":"gap","batches":n}).to_string().into())).await{return;},Err(_)=>return},
            input=incoming.next()=>match input{Some(Ok(Message::Close(_)))|None|Some(Err(_))=>return,_=>{}}
        }
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    let mut port = 8088u16;
    let mut assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/dist");
    let mut duration = None;
    let mut record = std::env::temp_dir().join(format!(
        "activity-demo-{}-{}.mcap",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
    ));
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => port = args.next().ok_or("missing port")?.parse()?,
            "--assets" => assets = args.next().ok_or("missing assets")?.into(),
            "--record" => record = args.next().ok_or("missing record path")?.into(),
            "--duration" => duration = Some(args.next().ok_or("missing duration")?.parse::<u64>()?),
            "--help" => {
                println!(
                    "activity-demo [--port 8088] [--assets PATH] [--record PATH] [--duration SECONDS]"
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    if !assets.join("index.html").is_file() {
        return Err(
            "build apps/activity-demo/frontend first (npm ci && npm run build), or pass --assets"
                .into(),
        );
    }
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let feed = Arc::new(Feed::new());
    let mut runtime = Runtime::start(
        Config {
            blocks: 128,
            encoded_buffers: 68,
            sink_queue: 32,
            ..Default::default()
        },
        vec![
            Box::new(McapSink::create(&record)?),
            Box::new(LiveSink(feed.clone())),
        ],
    )?;
    let handle = runtime.handle();
    let dispatch = tracing::Dispatch::new(handle.subscriber());
    let (senders, receivers): (Vec<_>, Vec<_>) = (0..NAMES.len()).map(|_| bounded(32)).unzip();
    let broker = Arc::new(Broker {
        sleep: AtomicU64::new(
            SleepSettings {
                mu_ms: 170,
                sigma_ms: 50,
            }
            .pack(),
        ),
        mailboxes: senders,
        next: AtomicU64::new(1),
        published: AtomicU64::new(0),
        received: AtomicU64::new(0),
        dropped: AtomicU64::new(0),
        round: AtomicU64::new(0),
    });
    let stop = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));
    let mut workers = Vec::new();
    for (index, rx) in receivers.into_iter().enumerate() {
        let b = broker.clone();
        let h = handle.clone();
        let d = dispatch.clone();
        let s = stop.clone();
        workers.push(
            thread::Builder::new()
                .name(NAMES[index].into())
                .spawn(move || worker(index, rx, b, h, d, s))?,
        );
    }
    let (frequency, mut frequency_rx) = tokio::sync::watch::channel(0.6f64);
    let app = Router::new()
        .route("/stream", get(stream))
        .route("/api/status", get(status))
        .route("/api/control", post(control))
        .route("/api/greet", post(greet))
        .route("/api/sleep", post(configure_sleep))
        .route("/api/playground", post(configure_playground))
        .with_state(App {
            frequency,
            feed,
            broker: broker.clone(),
            paused: paused.clone(),
            handle,
            recording: record.to_string_lossy().into_owned().into(),
        })
        .fallback_service(ServeDir::new(assets));
    let b = broker.clone();
    let p = paused.clone();
    let scheduler = tokio::spawn(async move {
        b.kick();
        loop {
            let period = Duration::from_secs_f64(1.0 / *frequency_rx.borrow_and_update());
            tokio::select! {
                changed = frequency_rx.changed() => { if changed.is_err() { break; } }
                _ = tokio::time::sleep(period) => { if !p.load(Ordering::Acquire) { b.kick(); } }
            }
        }
    });
    println!("Activity demo: http://{}", listener.local_addr()?);
    println!("Recording: {}", record.display());
    let result: Result<()> = tokio::select! {r=axum::serve(listener,app)=>r.map_err(Into::into),r=tokio::signal::ctrl_c()=>r.map_err(Into::into),_=async{match duration{Some(s)=>tokio::time::sleep(Duration::from_secs(s)).await,None=>std::future::pending().await}}=>Ok(())};
    scheduler.abort();
    stop.store(true, Ordering::Release);
    let mut worker_error = None;
    for w in workers {
        match w.join() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                worker_error = Some(e);
            }
            Err(_) => {
                worker_error = Some("worker panicked".into());
            }
        }
    }
    let report = runtime.shutdown()?;
    println!("{report:#?}");
    result?;
    if let Some(e) = worker_error {
        return Err(e);
    }
    if report.sinks.iter().any(|s| s.error.is_some()) {
        return Err("output failed; see shutdown report".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};
    #[test]
    fn frequency_validation() {
        for (frequency_hz, valid) in [
            (0.1, true),
            (20.0, true),
            (0.0, false),
            (20.1, false),
            (f64::NAN, false),
            (f64::INFINITY, false),
        ] {
            assert_eq!(
                PlaygroundSettings {
                    sleep: SleepSettings {
                        mu_ms: 170,
                        sigma_ms: 50
                    },
                    frequency_hz
                }
                .valid(),
                valid
            );
        }
    }
    #[test]
    fn gaussian_sampling_and_bounds() {
        let mut rng = StdRng::seed_from_u64(42);
        let fixed = SleepSettings {
            mu_ms: 250,
            sigma_ms: 0,
        };
        assert_eq!(SleepSettings::unpack(fixed.pack()), fixed);
        for _ in 0..100 {
            assert_eq!(fixed.sample(&mut rng), 250);
        }
        let settings = SleepSettings {
            mu_ms: 1000,
            sigma_ms: 100,
        };
        let samples: Vec<f64> = (0..50_000)
            .map(|_| settings.sample(&mut rng) as f64)
            .collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let sigma =
            (samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64).sqrt();
        assert!((mean - 1000.0).abs() < 2.0);
        assert!((sigma - 100.0).abs() < 2.0);
        let extreme = SleepSettings {
            mu_ms: 0,
            sigma_ms: 1000,
        };
        for _ in 0..10_000 {
            assert!(extreme.sample(&mut rng) <= 5000);
        }
        assert!(
            !SleepSettings {
                mu_ms: 2001,
                sigma_ms: 0
            }
            .valid()
        );
        assert!(
            !SleepSettings {
                mu_ms: 0,
                sigma_ms: 1001
            }
            .valid()
        );
    }
}
