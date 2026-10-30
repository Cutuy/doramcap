//! Run one native monitor from pbtxt; stop cooperatively after readiness or Ctrl-C.
use std::time::Duration;
use workload_supervisor::{SupervisorRunner, parse_run};
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: supervisor_monitor RUN.pbtxt OBSERVE_SECONDS".into());
    }
    let run = parse_run(&std::fs::read_to_string(&args[1])?)?;
    let seconds = args[2].parse::<u64>()?;
    let (ready, notification) = tokio::sync::oneshot::channel();
    let stop = async {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = async { if notification.await.is_ok() { tokio::time::sleep(Duration::from_secs(seconds)).await; } else { std::future::pending::<()>().await; } } => {},
        }
    };
    let report = SupervisorRunner::new()
        .run_with_ready(
            run,
            stop,
            |e: &orchestrator::Event| {
                eprintln!("{} {} {:?}", e.invocation.id, e.invocation.step, e.state);
            },
            Some(ready),
        )
        .await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.successful() {
        return Err("monitor run failed; inspect outputs and ownership.pb".into());
    }
    Ok(())
}
