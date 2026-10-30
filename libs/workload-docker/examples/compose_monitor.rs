//! Monitor-only Compose parity run with the shared recording policy.
use std::{rc::Rc, time::Duration};
use workload_docker::{
    compose::DockerAdapter,
    workflows::{
        recorded_compose::{ComposeRun, ComposeRunner},
        system_apps::{CpuMonitor, SystemAppsConfig},
    },
};
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("usage: compose_monitor ID NEW_OUTPUT_DIRECTORY IMAGE OBSERVE_SECONDS".into());
    }
    let adapter = Rc::new(DockerAdapter::connect("/var/run/docker.sock").await?);
    let spec = ComposeRun {
        filesystem: None,
        id: args[1].clone(),
        artifacts: args[2].clone().into(),
        recovery_anchor_dir: None,
        projects: vec![],
        first_party: vec![],
        system_apps: SystemAppsConfig {
            cpu_monitor: Some(CpuMonitor {
                image: args[3].clone(),
            }),
            ..Default::default()
        },
        readiness_seconds: 30,
        stop_seconds: 15,
    };
    let seconds = args[4].parse::<u64>()?;
    let (ready, notification) = tokio::sync::oneshot::channel();
    let stop = async {
        tokio::select! {
            _=tokio::signal::ctrl_c()=>{},
            _=async { if notification.await.is_ok() {tokio::time::sleep(Duration::from_secs(seconds)).await;} else {std::future::pending::<()>().await;} }=>{},
        }
    };
    let report = ComposeRunner::new(adapter)
        .run_with_ready(
            spec,
            stop,
            |e: &orchestrator::Event| {
                eprintln!("{} {} {:?}", e.invocation.id, e.invocation.step, e.state);
            },
            Some(ready),
        )
        .await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.successful() {
        return Err("monitor run failed; see report.json".into());
    }
    Ok(())
}
