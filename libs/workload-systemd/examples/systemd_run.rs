//! Run protobuf-text `RunPlan` and `SystemdPlan` files until Ctrl-C, then print the report.
//! Usage (as root): `systemd_run RUN.pbtxt PLAN.pbtxt`

/// Decode strict protobuf text from the file at argument `index` into message `name`.
fn read<M: prost::Message + Default>(index: usize, name: &str) -> Result<M, String> {
    let error = |e: &dyn std::fmt::Display| e.to_string();
    let path = std::env::args()
        .nth(index)
        .ok_or("usage: systemd_run RUN.pbtxt PLAN.pbtxt")?;
    let text = std::fs::read_to_string(path).map_err(|e| error(&e))?;
    let pool = prost_reflect::DescriptorPool::decode(
        include_bytes!(concat!(env!("OUT_DIR"), "/systemd_plan.bin")).as_slice(),
    )
    .map_err(|e| error(&e))?;
    let descriptor = pool
        .get_message_by_name(name)
        .ok_or_else(|| format!("{name} descriptor absent"))?;
    prost_reflect::DynamicMessage::parse_text_format(descriptor, &text)
        .map_err(|e| error(&e))?
        .transcode_to()
        .map_err(|e| error(&e))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), String> {
    if std::env::args().nth(1).as_deref() == Some("--cleanup") {
        let repository = std::env::args()
            .nth(2)
            .ok_or("usage: systemd_run --cleanup REPOSITORY")?;
        return workload_systemd::SystemdRunner::new()
            .cleanup(std::path::Path::new(&repository))
            .await;
    }
    let review = std::env::args().nth(1).as_deref() == Some("--review");
    let offset = usize::from(review);
    let run: workload_systemd::RunPlan = read(1 + offset, "doramcap.launch.v1.RunPlan")?;
    let plan: workload_systemd::SystemdPlan =
        read(2 + offset, "doramcap.systemd_plan.v1.SystemdPlan")?;
    if review {
        let (snapshot, warnings) = workload_systemd::review(&run, &plan).await?;
        println!(
            "{}",
            serde_json::json!({"snapshot": snapshot, "warnings": warnings})
        );
        return Ok(());
    }
    let stop = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    let (ready, readiness) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        if readiness.await.is_ok() {
            eprintln!("READY");
        }
    });
    let report = workload_systemd::SystemdRunner::new()
        .run(run, plan, stop, |_: &orchestrator::Event| {}, Some(ready))
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
