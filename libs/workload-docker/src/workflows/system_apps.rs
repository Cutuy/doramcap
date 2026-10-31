//! Supporting-app preparation, concurrent launch, readiness and recording checks.
//! Application-specific templates and policy stay behind these observable steps.
use crate::compose::{adapter::private_write, *};
use orchestrator::Step;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
pub use workload_lifecycle::recording::{
    FinalizeRecordings, ReadinessInput, Recording, WaitForRecordings,
};

include!(concat!(env!("OUT_DIR"), "/doramcap.system_apps.v1.rs"));

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
