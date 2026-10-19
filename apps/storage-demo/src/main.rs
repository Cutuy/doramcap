mod inspect;
mod metrics;
use axum::{
    Json, Router,
    extract::{Path as RoutePath, State},
    http::StatusCode,
    response::{Html, IntoResponse, Sse, sse::Event},
    routing::{get, post},
};
use prost::Message as _;
use serde_json::{Value, json};
use std::{
    convert::Infallible,
    fs::File,
    io::{self, Read},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use storage_persistence::mcap::{Channel, McapWriter, Message, Schema};
use telemetry_protocol::pb::{self, packet::Body};
use tokio::sync::watch;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
struct Options {
    monitor: PathBuf,
    backend: String,
    root: PathBuf,
    port: u16,
    direct: bool,
    seconds: u64,
    start_paused: bool,
}
fn options() -> Result<Options> {
    let mut o = Options {
        monitor: std::env::current_exe()?.with_file_name("system-monitor"),
        backend: "synthetic".into(),
        root: std::env::temp_dir().join(format!("doramcap-storage-demo-{}", now_ns())),
        port: 8091,
        direct: true,
        seconds: 0,
        start_paused: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--start-paused" {
            o.start_paused = true;
            continue;
        }
        if arg == "--buffered" {
            o.direct = false;
            continue;
        }
        if arg == "--help" {
            println!(
                "storage-demo [--monitor PATH] [--backend synthetic|bpf] [--dir NEW_DIR] [--port 8091] [--seconds N] [--buffered] [--start-paused]"
            );
            std::process::exit(0);
        }
        let value = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--monitor" => o.monitor = value.into(),
            "--backend" => o.backend = value,
            "--dir" => o.root = value.into(),
            "--port" => o.port = value.parse()?,
            "--seconds" => o.seconds = value.parse()?,
            _ => return Err(format!("unknown option {arg}").into()),
        }
    }
    if !matches!(o.backend.as_str(), "synthetic" | "bpf") {
        return Err("backend must be synthetic or bpf".into());
    }
    Ok(o)
}
fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}
struct Control {
    action: String,
    count: usize,
    interval_ms: u64,
}
#[derive(Clone)]
struct Web {
    snapshots: watch::Receiver<String>,
    commands: SyncSender<Control>,
    root: PathBuf,
    sealed: Arc<AtomicU64>,
}
async fn events(State(s): State<Web>) -> impl IntoResponse {
    let stream = futures_util::stream::unfold((s.snapshots, true), |(mut rx, first)| async move {
        if !first && rx.changed().await.is_err() {
            return None;
        }
        let data = rx.borrow_and_update().clone();
        Some((
            Ok::<_, Infallible>(Event::default().data(data)),
            (rx, false),
        ))
    });
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}
async fn control(State(s): State<Web>, Json(body): Json<Value>) -> StatusCode {
    let Some(action) = body["action"].as_str() else {
        return StatusCode::BAD_REQUEST;
    };
    if ![
        "burst",
        "large",
        "flush",
        "rotate",
        "restart",
        "stop",
        "probe_burst",
        "cancel_burst",
    ]
    .contains(&action)
    {
        return StatusCode::BAD_REQUEST;
    }
    let current: Value = serde_json::from_str(&s.snapshots.borrow()).unwrap();
    if action != "restart" && current["status"] != "live" {
        return StatusCode::CONFLICT;
    }
    let count = body["count"].as_u64().unwrap_or(32);
    let interval_ms = body["intervalMs"].as_u64().unwrap_or(5);
    if action == "probe_burst" && (!(1..=256).contains(&count) || interval_ms > 1000) {
        return StatusCode::BAD_REQUEST;
    }
    match s.commands.try_send(Control {
        action: action.to_owned(),
        count: count as usize,
        interval_ms,
    }) {
        Ok(()) => StatusCode::ACCEPTED,
        Err(mpsc::TrySendError::Full(_)) => StatusCode::TOO_MANY_REQUESTS,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}
async fn download(State(s): State<Web>, RoutePath(index): RoutePath<u64>) -> impl IntoResponse {
    if index >= s.sealed.load(Ordering::Acquire) {
        return (StatusCode::NOT_FOUND, Vec::new());
    }
    match std::fs::read(s.root.join(format!("capture-{index:04}.mcap"))) {
        Ok(bytes) => (StatusCode::OK, bytes),
        Err(_) => (StatusCode::NOT_FOUND, Vec::new()),
    }
}
struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        // SIGTERM lets the monitor and an optional sudo launcher stop their children.
        // SAFETY: this is the PID of the owned, unreaped child process.
        unsafe {
            libc::kill(self.0.id() as i32, libc::SIGTERM);
        }
        for _ in 0..100 {
            if matches!(self.0.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn open(o: &Options, index: u64) -> io::Result<File> {
    File::options()
        .write(true)
        .create_new(true)
        .custom_flags(if o.direct { libc::O_DIRECT } else { 0 })
        .open(o.root.join(format!("capture-{index:04}.mcap")))
}
fn packet(seq: u32, large: bool) -> pb::Packet {
    pb::Packet {
        body: Some(Body::Snapshot(pb::SnapshotFrame {
            version: 2,
            source: if large {
                "probe-".repeat(16384)
            } else {
                format!("heartbeat-{}", seq % 7)
            },
            snapshot: Some(pb::Snapshot {
                schema_version: 2,
                sequence: seq as u64,
                capture_unix_ns: now_ns(),
                ..Default::default()
            }),
            ..Default::default()
        })),
    }
}
fn record(writer: &mut McapWriter, p: &pb::Packet, channel: u16, seq: u32) -> Result<usize> {
    let len = p.encoded_len();
    let timestamp = now_ns();
    let message = Message {
        channel,
        sequence: seq,
        log_time: timestamp,
        publish_time: timestamp,
    };
    if len <= writer.max_inline_payload() {
        let mut r = writer.reserve(message, len)?;
        p.encode(&mut r.payload_mut())?;
        r.commit()?;
    } else {
        let mut r = writer.reserve_large(message, len)?;
        p.encode(&mut r)?;
        r.commit()?;
    }
    Ok(len)
}
fn owner(
    o: &Options,
    tx: watch::Sender<String>,
    commands: &mpsc::Receiver<Control>,
    stop: Arc<AtomicBool>,
    sealed: Arc<AtomicU64>,
) -> Result<bool> {
    let mut child = ChildGuard(
        Command::new(&o.monitor)
            .args(["--backend", &o.backend, "--records", "4096"])
            .stdout(Stdio::piped())
            .spawn()?,
    );
    let mut stdout = child.0.stdout.take().unwrap();
    let (input_tx, input) = mpsc::sync_channel(16);
    let drops = Arc::new(AtomicU64::new(0));
    let input_drops = drops.clone();
    std::thread::spawn(move || {
        let result = (|| -> io::Result<()> {
            loop {
                let mut prefix = [0; 4];
                stdout.read_exact(&mut prefix)?;
                let len = u32::from_le_bytes(prefix) as usize;
                if len > 1024 * 1024 {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                let mut bytes = vec![0; len];
                stdout.read_exact(&mut bytes)?;
                match input_tx.try_send(Ok(bytes)) {
                    Ok(()) => (),
                    Err(mpsc::TrySendError::Full(_)) => {
                        input_drops.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(mpsc::TrySendError::Disconnected(_)) => return Ok(()),
                }
            }
        })();
        if let Err(e) = result {
            let _ = input_tx.send(Err(e));
        }
    });
    let schema = Schema {
        name: telemetry_protocol::MCAP_SCHEMA_NAME,
        encoding: "protobuf",
        data: telemetry_protocol::FILE_DESCRIPTOR_SET,
    };
    let channels = [
        Channel {
            id: 1,
            topic: "demo/cpu",
            encoding: "protobuf",
        },
        Channel {
            id: 2,
            topic: "demo/heartbeat",
            encoding: "protobuf",
        },
        Channel {
            id: 3,
            topic: "demo/large-probe",
            encoding: "protobuf",
        },
    ];
    let mut writer = if o.direct {
        McapWriter::direct(open(o, 0)?, 8, 65536, schema, &channels, 4096)?
    } else {
        McapWriter::new(open(o, 0)?, 8, 65536, schema, &channels)?
    };
    let session = now_ns().to_string();
    let mut restart = false;
    let start = Instant::now();
    let mut heartbeat = Instant::now();
    let mut publish = Instant::now();
    let mut rng = now_ns();
    let (mut seq, mut cpu, mut beats, mut probes, mut index) = (0u32, 0u64, 0u64, 0u64, 0u64);
    let mut inspector = inspect::Inspector::new();
    let mut disk = json!(null);
    let mut last_sealed = json!(null);
    let mut recent = std::collections::VecDeque::with_capacity(48);
    let mut last_cpu = json!(null);
    let mut receipts = std::collections::VecDeque::with_capacity(8);
    let mut timing = metrics::CpuTiming::default();
    let mut latency = metrics::Window::default();
    let mut last_event = 0;
    let mut lost_events = 0;
    let mut burst_remaining = 0usize;
    let mut burst_interval = Duration::from_millis(5);
    let mut next_probe = Instant::now();
    while !stop.load(Ordering::Relaxed) && (o.seconds == 0 || start.elapsed().as_secs() < o.seconds)
    {
        let mut pending = Vec::with_capacity(32);
        for _ in 0..8 {
            match input.try_recv() {
                Ok(Ok(bytes)) => {
                    let p = pb::Packet::decode(bytes.as_slice())?;
                    if let Some(Body::Snapshot(f)) = &p.body
                        && let Some(s) = &f.snapshot
                    {
                        timing.observe(
                            s.sequence,
                            s.monotonic_end_ns,
                            start.elapsed().as_nanos() as u64,
                        );
                        last_cpu = json!({"sequence":s.sequence,"rows":s.records.len(),"dropped":s.dropped_batches,"source":f.source});
                    }
                    pending.push((p, 1, "CPU"));
                    cpu += 1;
                }
                Ok(Err(e)) => return Err(format!("CPU monitor input ended: {e}").into()),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("CPU monitor disconnected".into());
                }
            }
        }
        let mut action = None;
        let mut manual_kind = None;
        let mut manual_rows = Vec::with_capacity(16);
        let mut manual_us = 0u128;
        if let Ok(command) = commands.try_recv() {
            match command.action.as_str() {
                "burst" => {
                    manual_kind = Some("16 heartbeats");
                    for _ in 0..16 {
                        pending.push((packet(seq, false), 2, "Manual heartbeat"));
                        beats += 1;
                    }
                }
                "probe_burst" => {
                    burst_remaining = command.count;
                    burst_interval = Duration::from_millis(command.interval_ms);
                    next_probe = Instant::now();
                }
                "cancel_burst" => burst_remaining = 0,
                "large" => {
                    manual_kind = Some("96 KiB probe");
                    pending.push((packet(seq, true), 3, "Manual probe"));
                    probes += 1;
                }
                _ => action = Some(command.action),
            }
        }
        if burst_remaining > 0
            && Instant::now() >= next_probe
            && !matches!(action.as_deref(), Some("stop" | "restart"))
        {
            pending.push((packet(seq, true), 3, "Large probe"));
            probes += 1;
            burst_remaining -= 1;
            next_probe = Instant::now() + burst_interval;
        }
        if Instant::now() >= heartbeat {
            pending.push((packet(seq, false), 2, "Heartbeat"));
            beats += 1;
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            heartbeat = Instant::now() + Duration::from_millis(17 + rng % 380);
        }
        for (p, channel, kind) in pending {
            let encode_start = Instant::now();
            let len = record(&mut writer, &p, channel, seq)?;
            if kind.starts_with("Manual") {
                manual_us += encode_start.elapsed().as_micros();
                manual_rows.push((seq, len));
            }
            if recent.len() == 48 {
                recent.pop_front();
            }
            recent.push_back(json!({"sequence":seq,"kind":kind,"bytes":len,"timeMs":start.elapsed().as_millis()}));
            seq = seq.wrapping_add(1);
        }
        if let Some(kind) = manual_kind {
            if receipts.len() == 8 {
                receipts.pop_front();
            }
            receipts.push_back(json!({"kind":kind,"count":manual_rows.len(),
                "bytes":manual_rows.iter().map(|(_,len)|len).sum::<usize>(),
                "first":manual_rows.first().map(|(seq,_)|seq),"last":manual_rows.last().map(|(seq,_)|seq),
                "encodeMs":manual_us as f64/1000.0,"timeMs":start.elapsed().as_millis()}));
        }
        if let Some(action) = action {
            if action == "stop" {
                break;
            }
            if action == "restart" {
                restart = true;
                break;
            }
            if action == "flush" {
                writer.flush_written()?;
            }
            if action == "rotate" {
                writer.rotate(open(o, index + 1)?)?;
                last_sealed = inspector.read(
                    &o.root.join(format!("capture-{index:04}.mcap")),
                    index,
                    true,
                )?;
                sealed.store(index + 1, Ordering::Release);
                index += 1;
                inspector = inspect::Inspector::new();
                disk = json!(null);
            }
        }
        writer.poll(Instant::now())?;
        let seen_event = last_event;
        for event in writer.write_events().filter(|e| e.sequence > seen_event) {
            if event.sequence > last_event + 1 {
                lost_events += event.sequence - last_event - 1;
            }
            last_event = event.sequence;
            if event.result >= 0
                && let Some(us) = event.latency_us
            {
                latency.push(us as f64 / 1000.0);
            }
        }
        if Instant::now() >= publish {
            let d = writer.diagnostics();
            if d.outstanding == 0 {
                disk = inspector.read(
                    &o.root.join(format!("capture-{index:04}.mcap")),
                    index,
                    false,
                )?;
            }
            let events:Vec<_>=writer.write_events().map(|e|json!({"id":e.sequence,"file":e.file,"us":e.elapsed_us,"offset":e.offset,"len":e.len,"slot":e.slot,"result":e.result,"latencyUs":e.latency_us,"kind":if e.completion {"CQE"}else{"SQE queued"}})).collect();
            tx.send_replace(json!({"status":"live","session":session,"backend":o.backend,"direct":o.direct,"elapsedMs":start.elapsed().as_millis(),"cpu":cpu,"heartbeats":beats,"probes":probes,"inputDrops":drops.load(Ordering::Relaxed),"lastCpu":last_cpu,
                "combining":{"used":d.combining_bytes,"capacity":d.combining_capacity,"ageMs":d.combining_age_us as f64/1000.0,"deadlineMs":100},
                "pool":{"free":d.buffers_available,"total":d.buffers_total},"ring":{"outstanding":d.outstanding,"submitted":d.submitted_operations,"completions":d.completions,"accepted":d.accepted_bytes,"completed":d.completed_bytes},
                "timing":timing.json(),"latency":latency.json(),"lostTimingEvents":lost_events,"burstRemaining":burst_remaining,
                "receipts":receipts,"messages":recent,"events":events,"file":disk,"lastSealed":last_sealed}).to_string());
            publish = Instant::now() + Duration::from_millis(40);
        }
        if burst_remaining > 0 && burst_interval.is_zero() {
            std::thread::yield_now();
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let final_diagnostics = writer.finish_with_diagnostics()?;
    sealed.store(index + 1, Ordering::Release);
    let mut final_state: Value = serde_json::from_str(&tx.borrow())?;
    final_state["status"] = json!("sealed");
    final_state["burstRemaining"] = json!(0);
    final_state["combining"]["used"] = json!(0);
    final_state["combining"]["ageMs"] = json!(0);
    final_state["pool"]["free"] = json!(final_diagnostics.buffers_available);
    final_state["ring"] = json!({"outstanding":final_diagnostics.outstanding,"submitted":final_diagnostics.submitted_operations,"completions":final_diagnostics.completions,"accepted":final_diagnostics.accepted_bytes,"completed":final_diagnostics.completed_bytes});
    final_state["file"] = inspector.read(
        &o.root.join(format!("capture-{index:04}.mcap")),
        index,
        true,
    )?;
    tx.send_replace(final_state.to_string());
    Ok(restart)
}
/// The root is exclusively created by this process. Remove only its capture files.
fn clear_captures(root: &std::path::Path) -> io::Result<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let is_capture = name
            .strip_prefix("capture-")
            .and_then(|n| n.strip_suffix(".mcap"))
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        if is_capture && entry.file_type()?.is_file() {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let o = options()?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, o.port)).await?;
    std::fs::create_dir(&o.root)?;
    println!(
        "Storage demo: http://{} · files: {} · CPU: {}",
        listener.local_addr()?,
        o.root.display(),
        o.backend
    );
    let (tx, rx) =
        watch::channel(json!({"status":if o.start_paused {"idle"} else {"starting"}}).to_string());
    let (commands, command_rx) = mpsc::sync_channel(16);
    let stop = Arc::new(AtomicBool::new(false));
    let sealed = Arc::new(AtomicU64::new(0));
    let state = Web {
        snapshots: rx,
        commands,
        root: o.root.clone(),
        sealed: sealed.clone(),
    };
    let owner_stop = stop.clone();
    let errors = tx.clone();
    let keepalive = tx.clone();
    let worker = std::thread::spawn(move || {
        let mut restart = !o.start_paused;
        while !owner_stop.load(Ordering::Relaxed) {
            if !restart {
                match command_rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(command) if command.action == "restart" => restart = true,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    _ => continue,
                }
            }
            if restart {
                sealed.store(0, Ordering::Release);
                tx.send_replace(json!({"status":"starting"}).to_string());
                let result = clear_captures(&o.root).map_err(Into::into).and_then(|()| {
                    owner(
                        &o,
                        tx.clone(),
                        &command_rx,
                        owner_stop.clone(),
                        sealed.clone(),
                    )
                });
                match result {
                    Ok(requested) => restart = requested,
                    Err(e) => {
                        errors.send_replace(
                            json!({"status":"error","error":e.to_string()}).to_string(),
                        );
                        eprintln!("storage demo: {e}");
                        restart = false;
                    }
                }
            }
        }
    });
    let app = Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../web/index.html")) }),
        )
        .route("/events", get(events))
        .route("/api/control", post(control))
        .route("/files/{index}", get(download))
        .route(
            "/api/state",
            get(|State(s): State<Web>| async move {
                Json(serde_json::from_str::<Value>(&s.snapshots.borrow()).unwrap())
            }),
        )
        .with_state(state);
    let shutdown_stop = stop.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            shutdown_stop.store(true, Ordering::Relaxed);
            drop(keepalive);
        })
        .await?;
    stop.store(true, Ordering::Relaxed);
    worker.join().map_err(|_| "writer thread panicked")?;
    Ok(())
}
