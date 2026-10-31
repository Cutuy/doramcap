//! Local gallery hosting server.
use project_explorer_server::{
    api, lease::ManagerLease, lifecycle::Status, manager::Manager, registry::Registry,
    runtime::Service,
};
use std::{net::SocketAddr, path::PathBuf, rc::Rc, time::Duration};
use workload_docker::compose::DockerAdapter;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Options {
    workspace: PathBuf,
    data: PathBuf,
    assets: PathBuf,
    socket: String,
    bind: SocketAddr,
    origin: String,
    idle: Duration,
    heartbeat: Duration,
    enabled_apps: Vec<String>,
}
impl Options {
    fn parse() -> Result<Self> {
        let mut workspace = None;
        let mut data = None;
        let mut assets = None;
        let mut origin = None;
        let mut socket = "/var/run/docker.sock".to_owned();
        let mut bind = "127.0.0.1:8095".parse()?;
        let mut idle = Duration::from_secs(120);
        let mut heartbeat = Duration::from_secs(15);
        let mut enabled_apps = Vec::new();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--enable-app" => enabled_apps.push(args.next().ok_or("missing app id")?),
                "--workspace" => workspace = Some(PathBuf::from(args.next().ok_or("missing workspace")?)),
                "--data" => data = Some(PathBuf::from(args.next().ok_or("missing data directory")?)),
                "--assets" => assets = Some(PathBuf::from(args.next().ok_or("missing assets")?)),
                "--origin" => origin = Some(args.next().ok_or("missing public origin")?),
                "--socket" => socket = args.next().ok_or("missing Docker socket")?,
                "--bind" => bind = args.next().ok_or("missing bind address")?.parse()?,
                "--idle-seconds" => idle = Duration::from_secs(args.next().ok_or("missing idle timeout")?.parse()?),
                "--heartbeat-seconds" => heartbeat = Duration::from_secs(args.next().ok_or("missing heartbeat interval")?.parse()?),
                "--help" => return Err("usage: project-explorer-server --workspace PATH --data PATH --assets PATH --origin HTTP_ORIGIN [--socket PATH] [--bind IP:PORT] [--idle-seconds N] [--heartbeat-seconds N] [--enable-app APP_ID]...".into()),
                _ => return Err(format!("unknown argument: {arg}").into()),
            }
        }
        let origin: String = origin.ok_or("--origin is required")?;
        let uri: axum::http::Uri = origin.parse()?;
        let valid = matches!(uri.scheme_str(), Some("http" | "https"))
            && uri
                .authority()
                .is_some_and(|authority| !authority.as_str().contains('@'))
            && uri.path() == "/"
            && uri.query().is_none()
            && format!(
                "{}://{}",
                uri.scheme_str().unwrap_or_default(),
                uri.authority().map(|a| a.as_str()).unwrap_or_default()
            ) == origin;
        if !valid {
            return Err("--origin must be an HTTP(S) origin without a path".into());
        }
        if heartbeat.is_zero() || heartbeat >= idle {
            return Err("heartbeat interval must be positive and less than idle timeout".into());
        }
        Ok(Self {
            workspace: workspace.ok_or("--workspace is required")?,
            data: data.ok_or("--data is required")?,
            assets: assets.ok_or("--assets is required")?,
            socket,
            bind,
            origin,
            idle,
            heartbeat,
            enabled_apps,
        })
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let options = Options::parse()?;
    tokio::task::LocalSet::new().run_until(run(options)).await
}

async fn run(options: Options) -> Result<()> {
    let workspace = std::fs::canonicalize(options.workspace)?;
    let root = std::fs::canonicalize(options.data)?;
    let assets = std::fs::canonicalize(options.assets)?;
    if !assets.join("index.html").is_file() {
        return Err("gallery assets lack index.html".into());
    }
    let _lease = ManagerLease::acquire(&root)?;
    let registry = Registry::load(&workspace, &options.enabled_apps)?;
    let docker = Rc::new(DockerAdapter::connect(options.socket).await?);
    let manager = Manager::new(registry, options.idle);
    // SAFETY: geteuid/getegid have no arguments or side effects.
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
    let service = Service::spawn_local(manager, docker, root, uid, gid);
    let listener = tokio::net::TcpListener::bind(options.bind).await?;
    let addr = listener.local_addr()?;
    let maintenance = tokio::task::spawn_local({
        let service = service.clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                service.expire(std::time::Instant::now()).await;
            }
        }
    });
    println!("Gallery hosting API listening on http://{addr}");
    let closing = service.clone();
    let result = axum::serve(
        listener,
        api::router(
            service.clone(),
            options.origin,
            assets,
            options.heartbeat.as_secs(),
        ),
    )
    .with_graceful_shutdown(async move {
        shutdown_signal().await;
        closing.close_admission();
    })
    .await;
    maintenance.abort();
    let outcomes = service.shutdown().await;
    for (app, status) in outcomes {
        match status {
            Status::Blocked { error } => eprintln!("{app}: cleanup unverified: {error}"),
            Status::Stopped { error: Some(error) } => {
                eprintln!("{app}: finalization failed: {error}")
            }
            _ => {}
        }
    }
    result?;
    Ok(())
}

async fn shutdown_signal() {
    let terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate());
    if let Ok(mut terminate) = terminate {
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    } else {
        let _ = tokio::signal::ctrl_c().await;
    }
}
