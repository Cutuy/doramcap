mod support {
    pub mod docker_workflow;
}
use orchestrator::{Error, Result};
use std::path::Path;
use support::docker_workflow::experiment;
fn fail(e: impl std::fmt::Display) -> Error {
    Error::message(e.to_string())
}
fn ensure(ok: bool, message: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(fail(message)) }
}
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Arrange: isolated artifact directory and unique experiment identities.
    let args: Vec<_> = std::env::args().collect();
    ensure(
        args.len() == 3,
        "usage: docker_monitor IMAGE EXISTING_OUTPUT_DIRECTORY",
    )?;
    let root = Path::new(&args[2]).canonicalize().map_err(fail)?;
    let prefix = format!(
        "doramcap-e2e-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(fail)?
            .as_millis()
    );
    let mut reports = Vec::new();
    // Act: run two normal experiments, then fail an audit after readiness.
    for (index, inject) in [false, false, true].into_iter().enumerate() {
        reports.push(
            experiment(
                &args[1],
                &root,
                &format!("{prefix}-{index}"),
                inject,
                |_| {},
            )
            .await?,
        );
    }
    // Assert: each attempt owns fresh workload and recording identities.
    ensure(
        reports[0]["recording"]["container"] != reports[1]["recording"]["container"],
        "container reused across experiments",
    )?;
    ensure(
        reports[0]["recording"]["monitor_session"] != reports[1]["recording"]["monitor_session"],
        "monitor session reused",
    )?;
    tokio::fs::write(
        root.join("summary.json"),
        serde_json::to_vec_pretty(&reports).map_err(fail)?,
    )
    .await
    .map_err(fail)?;
    println!(
        "PASS: two native Docker experiments and injected-failure cleanup; artifacts {}",
        root.display()
    );
    Ok(())
}
