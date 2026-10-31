//! Local demo dashboard and bounded control bridge.
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use dora_node_api::{Event, StopCause, futures::StreamExt};
use eyre::Result;
use serde_json::Value;
use std::sync::{Arc, Mutex};
#[derive(Clone)]
struct App {
    view: Arc<Mutex<fault_demo::Display>>,
    controls: Arc<tokio::net::UnixDatagram>,
    control_path: std::path::PathBuf,
}
#[tokio::main(worker_threads = 2)]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let state_file = args.next().unwrap_or_else(|| "state.json".into());
    let control_path = args
        .next()
        .unwrap_or_else(|| "/tmp/fault-demo-controls.sock".into())
        .into();
    let (_node, mut events) = dora_app::environment::connect()?;
    if args.next().is_some() {
        eyre::bail!("unexpected argument");
    }
    let state = App {
        view: Default::default(),
        controls: Arc::new(tokio::net::UnixDatagram::unbound()?),
        control_path,
    };
    let view = state.view.clone();
    let router = Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../../frontend/index.html")) }),
        )
        .route(
            "/api/state",
            get(|State(app): State<App>| async move { Json(app.view.lock().unwrap().json()) }),
        )
        .route("/api/control", post(control))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(
        std::env::var("FAULT_DEMO_LISTEN").unwrap_or_else(|_| "127.0.0.1:8088".into()),
    )
    .await?;
    dora_app::environment::ready()?;
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    // Keep the subscription alive while forwarding controls between selects.
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            event=events.next()=>match event{Some(Event::Input{data,..})=>{if let Ok(bytes)=<&[u8]>::try_from(&data){match fault_protocol::decode_aggregation(bytes,Default::default()){Ok(frame)=>view.lock().unwrap().apply(frame),Err(e)=>eprintln!("invalid publication: {e}")}}},Some(Event::InputClosed{..})|Some(Event::Stop(StopCause::AllInputsClosed))=>{},Some(Event::Stop(_))|None=>break,_=>{}},
            _=&mut shutdown=>break,
        }
    }
    server.abort();
    std::fs::write(
        state_file,
        serde_json::to_vec(&view.lock().unwrap().json())?,
    )?;
    Ok(())
}
async fn control(State(app): State<App>, Json(value): Json<Value>) -> StatusCode {
    let source = value["source"].as_str().unwrap_or("");
    let mode = value["mode"].as_str().unwrap_or("");
    if !["a", "b", "all"].contains(&source)
        || !["healthy", "fail", "flap", "pause", "flood", "event"].contains(&mode)
    {
        return StatusCode::BAD_REQUEST;
    }
    let bytes = serde_json::to_vec(&serde_json::json!({"source":source,"mode":mode})).unwrap();
    if app.controls.try_send_to(&bytes, &app.control_path).is_ok() {
        app.view.lock().unwrap().command(source, mode);
        StatusCode::ACCEPTED
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
