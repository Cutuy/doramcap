//! Run a caller-supplied JSON request until Ctrl-C or an optional duration.
use std::{path::Path, rc::Rc, time::Duration};
use workload_docker::{
    compose::DockerAdapter,
    workflows::recorded_compose::{ComposeRun, ComposeRunner},
};
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        return Err("usage: compose_experiment REQUEST.json [SECONDS] [SOCKET]".into());
    }
    let request: ComposeRun = serde_json::from_slice(&tokio::fs::read(Path::new(&args[1])).await?)?;
    let seconds = args.get(2).map(|s| s.parse::<u64>()).transpose()?;
    let adapter = Rc::new(
        DockerAdapter::connect(
            args.get(3)
                .map(String::as_str)
                .unwrap_or("/var/run/docker.sock"),
        )
        .await?,
    );
    let mut manager = ComposeRunner::new(adapter);
    let stop = async {
        if let Some(seconds) = seconds {
            tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = tokio::time::sleep(Duration::from_secs(seconds)) => {} }
        } else {
            let _ = tokio::signal::ctrl_c().await;
        }
    };
    let report = manager
        .run(request, stop, |e: &orchestrator::Event| {
            eprintln!("{} {} {:?}", e.invocation.id, e.invocation.step, e.state);
        })
        .await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.successful() {
        return Err("experiment failed; see report diagnostics".into());
    }
    Ok(())
}
