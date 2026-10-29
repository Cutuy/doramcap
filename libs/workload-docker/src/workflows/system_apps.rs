//! Supporting-app preparation, concurrent launch, readiness and recording checks.
//! Application-specific templates and policy stay behind these observable steps.
use crate::compose::{adapter::private_write, error::invalid, *};
use futures_util::future::join_all;
use orchestrator::{LocalExecutor, Step};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    rc::Weak,
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

include!(concat!(env!("OUT_DIR"), "/doramcap.system_apps.v1.rs"));

impl SystemAppsConfig {
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(monitor) = &self.cpu_monitor {
            if monitor.allow_unconfined_seccomp == monitor.seccomp_profile.is_some() {
                return Err(invalid(
                    "select exactly one explicit seccomp profile or unconfined exception",
                ));
            }
            if monitor
                .seccomp_profile
                .as_ref()
                .is_some_and(|path| !Path::new(path).is_absolute())
            {
                return Err(invalid("seccomp profile path must be absolute"));
            }
            if !["", "bpf", "synthetic"].contains(&monitor.backend.as_str()) {
                return Err(invalid("unsupported monitor backend"));
            }
        }
        Ok(())
    }
}

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

/// Snapshot app templates and reviewed policies into an existing private run directory.
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
    input.config.validate()?;
    let artifacts = input.artifacts;
    let Some(monitor) = input.config.cpu_monitor else {
        return Ok(PreparedSystemApps::default());
    };
    let system_file = artifacts.join("system-services.yaml");
    let template: &[u8] = if monitor.backend == "synthetic" {
        include_bytes!("../../../../apps/system-monitor/deploy/recording-synthetic.yaml")
    } else {
        include_bytes!("../../../../apps/system-monitor/deploy/recording.yaml")
    };
    private_write(&system_file, template).await?;
    let seccomp = stage_seccomp(&monitor, &artifacts).await?;
    // SAFETY: process identity calls have no arguments or side effects.
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
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
                    "DORAMCAP_MONITOR_BACKEND".into(),
                    if monitor.backend.is_empty() {
                        "bpf".into()
                    } else {
                        monitor.backend
                    },
                ),
                (
                    "DORAMCAP_ARTIFACTS".into(),
                    artifacts.to_string_lossy().into_owned(),
                ),
                ("DORAMCAP_MONITOR_SECCOMP".into(), seccomp),
                ("DORAMCAP_MONITOR_UID".into(), uid.to_string()),
                ("DORAMCAP_MONITOR_GID".into(), gid.to_string()),
            ]),
        }],
        recordings: vec![Recording {
            stream: artifacts.join("cpu.pb"),
            recording: artifacts.join("cpu.mcap"),
        }],
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaunchInput {
    pub projects: Vec<ResolvedComposeProject>,
    pub recordings: Vec<Recording>,
    pub readiness_seconds: u64,
}

/// Launch independent supporting projects, then verify their output readiness.
/// All starts are awaited even after a sibling fails. Cancellation drops pending
/// CLI futures; the caller must retain every attempted plan for Docker cleanup.
/// Uses the same executor for child events and dependency/parent lineage.
pub struct LaunchSystemApps {
    pub executor: Weak<LocalExecutor>,
}
impl Step for LaunchSystemApps {
    type Input = LaunchInput;
    type Output = ();
    async fn run(&mut self, input: LaunchInput) -> orchestrator::Result<()> {
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| orchestrator::Error::message("system-app executor dropped"))?;
        let started = join_all(input.projects.into_iter().map(|plan| {
            executor.run::<StartCompose>(StartInput {
                plan,
                readiness_seconds: input.readiness_seconds,
            })
        }))
        .await;
        let mut readiness = orchestrator::Input::new(ReadinessInput {
            recordings: input.recordings,
            timeout_seconds: input.readiness_seconds,
        });
        for result in started {
            readiness = readiness.after(&result?);
        }
        executor.run::<WaitForSystemApps>(readiness).await?;
        Ok(())
    }
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

async fn stage_seccomp(monitor: &CpuMonitor, artifacts: &Path) -> Result<String> {
    use std::{io::Read, os::unix::fs::OpenOptionsExt};
    let Some(profile) = &monitor.seccomp_profile else {
        return Ok("seccomp=unconfined".into());
    };
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(profile)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("seccomp profile must be a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err(invalid("seccomp profile exceeds 1 MiB"));
    }
    if !serde_json::from_slice::<serde_json::Value>(&bytes)?.is_object() {
        return Err(invalid("seccomp profile must be a JSON object"));
    }
    let saved = artifacts.join("monitor-seccomp.json");
    private_write(&saved, &bytes).await?;
    Ok(format!(
        "seccomp={}",
        saved
            .to_str()
            .ok_or_else(|| invalid("seccomp path is not UTF-8"))?
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn monitor_policy_requires_one_explicit_choice() {
        // Arrange
        let mut config = SystemAppsConfig {
            cpu_monitor: Some(CpuMonitor {
                image: "test".into(),
                ..Default::default()
            }),
        };
        // Act / Assert
        assert!(config.validate().is_err());
        config.cpu_monitor.as_mut().unwrap().seccomp_profile = Some("/tmp/reviewed.json".into());
        assert!(config.validate().is_ok());
        config
            .cpu_monitor
            .as_mut()
            .unwrap()
            .allow_unconfined_seccomp = true;
        assert!(config.validate().is_err());
        config.cpu_monitor.as_mut().unwrap().seccomp_profile = None;
        assert!(config.validate().is_ok());
        config.cpu_monitor.as_mut().unwrap().backend = "unknown".into();
        assert!(config.validate().is_err());
        config.cpu_monitor.as_mut().unwrap().backend = "synthetic".into();
        config
            .cpu_monitor
            .as_mut()
            .unwrap()
            .allow_unconfined_seccomp = false;
        config.cpu_monitor.as_mut().unwrap().seccomp_profile = Some("relative.json".into());
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn reviewed_seccomp_is_copied_privately_and_rejects_symlinks() {
        // Arrange
        let root = std::env::temp_dir().join(format!(
            "monitor-policy-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir(&root).unwrap();
        let profile = root.join("reviewed.json");
        std::fs::write(&profile, br#"{"defaultAction":"SCMP_ACT_ERRNO"}"#).unwrap();
        let monitor = CpuMonitor {
            image: "test".into(),
            backend: "synthetic".into(),
            allow_unconfined_seccomp: false,
            seccomp_profile: Some(profile.to_str().unwrap().into()),
        };

        // Act
        let option = stage_seccomp(&monitor, &root).await.unwrap();
        // Assert
        let saved = root.join("monitor-seccomp.json");
        assert_eq!(option, format!("seccomp={}", saved.display()));
        assert_eq!(
            std::fs::read(&saved).unwrap(),
            std::fs::read(&profile).unwrap()
        );
        assert_eq!(
            std::fs::metadata(&saved).unwrap().permissions().mode() & 0o777,
            0o600
        );

        // Act / Assert: a symlink and malformed JSON are not accepted as policies.
        let link = root.join("link.json");
        std::os::unix::fs::symlink(&profile, &link).unwrap();
        let mut invalid = monitor.clone();
        invalid.seccomp_profile = Some(link.to_str().unwrap().into());
        assert!(stage_seccomp(&invalid, &root).await.is_err());
        std::fs::write(&profile, b"not json").unwrap();
        assert!(stage_seccomp(&monitor, &root).await.is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
