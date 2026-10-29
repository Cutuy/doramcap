//! Supporting-app preparation, concurrent launch, readiness and recording checks.
//! Application-specific templates and policy stay behind these observable steps.
use crate::compose::{adapter::private_write, error::invalid, *};
use futures_util::future::join_all;
use orchestrator::Step;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

include!(concat!(env!("OUT_DIR"), "/doramcap.system_apps.v1.rs"));

/// Prepared application-output paths. A growing stream plus an armed recording
/// establishes readiness; a terminal MCAP footer establishes finalization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recording {
    pub stream: PathBuf,
    pub recording: PathBuf,
}

/// Serializable result of system-app preparation, independent of live OS handles.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PreparedSystemApps {
    pub projects: Vec<ComposeProject>,
    pub recordings: Vec<Recording>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrepareInput {
    pub config: SystemAppsConfig,
    pub artifacts: PathBuf,
}

/// Snapshot app templates into an existing private run directory.
/// Performs no Docker launch; preparation failures leave only private files.
pub struct PrepareSystemApps;
impl Step for PrepareSystemApps {
    type Input = PrepareInput;
    type Output = PreparedSystemApps;
    async fn run(&mut self, input: PrepareInput) -> orchestrator::Result<PreparedSystemApps> {
        prepare(input)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
async fn prepare(input: PrepareInput) -> Result<PreparedSystemApps> {
    let artifacts = input.artifacts;
    let Some(monitor) = input.config.cpu_monitor else {
        return Ok(PreparedSystemApps::default());
    };
    let system_file = artifacts.join("system-services.yaml");
    let template = include_bytes!("../../../../apps/system-monitor/deploy/recording.yaml");
    private_write(&system_file, template).await?;
    Ok(PreparedSystemApps {
        projects: vec![ComposeProject {
            key: "system".into(),
            directory: artifacts.clone(),
            files: vec![system_file],
            env_files: vec![],
            profiles: vec![],
            services: vec![],
            completed_services: vec![],
            remove_volumes: false,
            environment: BTreeMap::from([
                ("DORAMCAP_MONITOR_IMAGE".into(), monitor.image),
                (
                    "DORAMCAP_ARTIFACTS".into(),
                    artifacts.to_string_lossy().into_owned(),
                ),
            ]),
        }],
        recordings: vec![Recording {
            stream: artifacts.join("cpu.pb"),
            recording: artifacts.join("cpu.mcap"),
        }],
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReadinessInput {
    pub recordings: Vec<Recording>,
    pub timeout_seconds: u64,
}
/// Await independent application-output readiness probes on the caller's task.
pub struct WaitForSystemApps;
impl Step for WaitForSystemApps {
    type Input = ReadinessInput;
    type Output = ();
    async fn run(&mut self, input: ReadinessInput) -> orchestrator::Result<()> {
        let results = join_all(
            input
                .recordings
                .iter()
                .map(|output| wait_ready(output, input.timeout_seconds)),
        )
        .await;
        for result in results {
            result.map_err(|e| orchestrator::Error::message(e.to_string()))?;
        }
        Ok(())
    }
}
/// Verify recordings after their writers stop. Probes run concurrently and never
/// stop containers or publish artifacts; those remain the enclosing workflow's job.
pub struct FinalizeSystemApps;
impl Step for FinalizeSystemApps {
    type Input = Vec<Recording>;
    type Output = ();
    async fn run(&mut self, outputs: Vec<Recording>) -> orchestrator::Result<()> {
        let results = join_all(
            outputs
                .iter()
                .map(|output| finalized_recording(&output.recording)),
        )
        .await;
        for result in results {
            result.map_err(|e| orchestrator::Error::message(e.to_string()))?;
        }
        Ok(())
    }
}

async fn wait_ready(output: &Recording, timeout: u64) -> Result<()> {
    let start = tokio::time::Instant::now();
    let mut previous = None;
    loop {
        if let Ok(metadata) = tokio::fs::metadata(&output.stream).await {
            if previous.is_some_and(|n| n > 0 && metadata.len() > n)
                && tokio::fs::try_exists(&output.recording).await?
            {
                return Ok(());
            }
            previous = Some(metadata.len());
        }
        if start.elapsed() > Duration::from_secs(timeout) {
            return Err(invalid(
                "system app did not advance its stream/arm MCAP recording",
            ));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
async fn finalized_recording(path: &Path) -> Result<()> {
    let mut file = tokio::fs::File::open(path).await?;
    let len = file.metadata().await?.len();
    if len < 45 {
        return Err(invalid("MCAP is truncated"));
    }
    let mut header = [0u8; 8];
    file.read_exact(&mut header).await?;
    file.seek(std::io::SeekFrom::End(-37)).await?;
    let mut tail = [0u8; 37];
    file.read_exact(&mut tail).await?;
    if &header != b"\x89MCAP0\r\n"
        || tail[29..] != header
        || tail[0] != 2
        || u64::from_le_bytes(tail[1..9].try_into().unwrap()) != 20
    {
        return Err(invalid("MCAP terminal footer/magic absent"));
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    if tokio::fs::metadata(path).await?.len() != len {
        return Err(invalid("recording changed after producer exit"));
    }
    Ok(())
}
