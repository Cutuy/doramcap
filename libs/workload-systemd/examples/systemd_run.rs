//! Run protobuf-text `RunPlan` and `SystemdPlan` files until Ctrl-C, then print the report.
//! Usage (as root): `systemd_run RUN.pbtxt PLAN.pbtxt`
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), String> {
    let read = |index: usize| -> Result<String, String> {
        let path = std::env::args()
            .nth(index)
            .ok_or("usage: systemd_run RUN.pbtxt PLAN.pbtxt")?;
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    };
    let run = workload_systemd::parse_run(&read(1)?)?;
    let plan = workload_systemd::parse_plan(&read(2)?)?;
    let stop = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    let report = workload_systemd::SystemdRunner::new()
        .run(run, plan, stop, |_: &orchestrator::Event| {}, None)
        .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    );
    if report.successful() {
        Ok(())
    } else {
        Err("run was not clean".into())
    }
}
