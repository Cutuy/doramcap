//! Retained resources from the first native CLI launch attempt through verified stop.
use super::{
    arguments, container,
    observation::{self, CliStreams, Events},
    views,
};
use crate::{HoloscanPlan, process::HelperProcess};
use bollard::models::ContainerInspectResponse;
use docker_engine::{Engine, docker_proto::VolumeMount};
use host_filesystem::{
    changes::{FilesystemSources, SourcesInput},
    filesystem_proto::DirectoryMount,
    overlay::OverlaySet,
};
use orchestrator::{Error, Step};
use std::{
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    rc::Rc,
    time::Duration,
};
use tokio::{process::Child, sync::Mutex, task::JoinHandle};
use workload_lifecycle::{capture::PrepareFilesystemSources, launch::RunPlan};

pub(super) const CIDFILE: &str = "holoscan-container.cid";
const CONTAINER_ID_BYTES: usize = 64;
const COMPONENT: &str = "holoscan-application";

pub(super) struct Live {
    pub engine: Engine,
    pub helper: Option<HelperProcess>,
    pub child: Option<Child>,
    pub cli_status: Option<ExitStatus>,
    pub cli_error: Option<String>,
    pub output: Option<CliStreams>,
    pub events: Option<Events>,
    pub container: Option<String>,
    pub inspection: Option<ContainerInspectResponse>,
    pub observed_mounts: Vec<DirectoryMount>,
    pub coverage: Vec<String>,
    pub sources_task: Option<JoinHandle<Result<FilesystemSources, String>>>,
    pub sources: Option<FilesystemSources>,
    pub sources_attempted: bool,
    pub source_error: Option<String>,
    pub system_sources: Option<SourcesInput>,
    pub attempted: bool,
    pub notes: Vec<String>,
}
impl Live {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine,
            helper: None,
            child: None,
            cli_status: None,
            cli_error: None,
            output: None,
            events: None,
            container: None,
            inspection: None,
            observed_mounts: Vec::new(),
            coverage: Vec::new(),
            sources_task: None,
            sources: None,
            sources_attempted: false,
            source_error: None,
            system_sources: None,
            attempted: false,
            notes: Vec::new(),
        }
    }

    pub fn begin_observation(&mut self, run: &RunPlan) -> Result<(), String> {
        let directory =
            compose_apps::compose::adapter::repository_scope(Path::new(&run.repository_root));
        self.events = Some(Events::start(
            self.engine.clone(),
            utf8(&directory)?.into(),
            run.id.clone(),
            Path::new(&run.artifacts).join(observation::EVENTS),
        )?);
        Ok(())
    }

    async fn launch(
        &mut self,
        run: RunPlan,
        plan: HoloscanPlan,
        overlays: OverlaySet,
        image: String,
    ) -> Result<String, String> {
        let mappings = views::mappings(&overlays, &run, &plan)?;
        let project = mappings[0];
        crate::routing::verify_native_daemon(&project.effective, &self.engine)
            .await
            .map_err(error)?;
        if self.events.is_none() {
            return Err("run event observation must begin before launch".into());
        }
        let cidfile = Path::new(&run.artifacts).join(CIDFILE);
        let (_watcher, mut files) =
            workload_lifecycle::files::watch_changes([Path::new(&run.artifacts)]).map_err(error)?;
        let mut changes = self.events.as_ref().expect("event owner").changes();
        self.helper = Some(
            HelperProcess::prepare(
                &self.engine,
                &run,
                compose_apps::cgroup::Role::User,
                COMPONENT,
            )
            .await?,
        );
        let helper = self.helper.as_ref().expect("helper owner");
        let mut command = helper.command(Path::new(&plan.cli))?;
        command
            .args(arguments::run(
                &run,
                &plan,
                &overlays,
                helper.cgroup_parent(),
                &cidfile,
            )?)
            .current_dir(&project.effective)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        crate::environment::configure_container(&mut command);
        self.attempted = true;
        self.child = Some(command.spawn().map_err(error)?);
        self.output = Some(CliStreams::start(
            self.child.as_mut().expect("child owner"),
            Path::new(&run.artifacts),
        )?);
        let observed = tokio::time::timeout(Duration::from_secs(run.readiness_seconds), async {
            loop {
                changes.borrow_and_update().clone()?;
                if let Some(id) = read_cid(&cidfile).await? {
                    let info = self.engine.client().inspect_container(&id, None).await.map_err(error)?;
                    container::verify_ownership(&info, &run)?;
                    if info.id.as_deref() != Some(&id) { return Err("native cidfile and Engine container identity differ".into()); }
                    // Retain proven ownership before checking image/mount/placement failures.
                    self.container = Some(id.clone());
                    self.inspection = Some(info.clone());
                    let helper = self.helper.as_ref().expect("helper owner");
                    self.observed_mounts = container::validate(&info, &run, &image, helper.cgroup_parent(), &overlays, &plan).await?;
                    let state = info.state.as_ref().ok_or("native application state is unavailable")?;
                    let running = state.running.ok_or("native application running state is unavailable")?;
                    let started = crate::container_started(state.started_at.as_deref());
                    let exited = state.status.as_ref().is_some_and(|status| status.to_string() == "exited");
                    self.inspection = Some(info);
                    if running || (started && exited) { return Ok(id); }
                    if self.cli_status.is_some() { return Err("native CLI exited before the application started".into()); }
                } else if self.cli_status.is_some() {
                    return Err("native CLI exited before an owned application container was established".into());
                }
                let child = self.child.as_mut().expect("retained child");
                let output_failure = self.output.as_ref().expect("retained output").failure();
                tokio::select! {
                    status = child.wait(), if self.cli_status.is_none() => {
                        self.cli_status = Some(status.map_err(error)?);
                    }
                    changed = changes.changed() => { changed.map_err(error)?; }
                    changed = workload_lifecycle::files::wait_for_change(&mut files) => { changed.map_err(error)?; }
                    error = output_failure => return Err(format!("native CLI output: {error}")),
                }
            }
        }).await.map_err(|_| "native application readiness deadline expired".to_owned())??;
        self.prepare_sources(&run, &plan, &overlays).await?;
        Ok(observed)
    }

    pub async fn prepare_sources(
        &mut self,
        run: &RunPlan,
        plan: &HoloscanPlan,
        overlays: &OverlaySet,
    ) -> Result<(), String> {
        if !self.sources_attempted {
            let input = self.source_input(run, plan, overlays).await?;
            let coverage = self.coverage.clone();
            self.sources_attempted = true;
            self.sources_task = Some(tokio::spawn(async move {
                let mut step = PrepareFilesystemSources;
                let mut sources = step.run(input).await.map_err(error)?;
                sources.add_coverage_notes(coverage);
                Ok(sources)
            }));
        }
        self.settle_sources().await?;
        if self.sources.is_some() {
            Ok(())
        } else {
            Err(self
                .source_error
                .clone()
                .unwrap_or_else(|| "native filesystem sources are unavailable".into()))
        }
    }

    pub async fn settle_sources(&mut self) -> Result<(), String> {
        if let Some(task) = &mut self.sources_task {
            let settled = task.await;
            self.sources_task.take();
            match settled.map_err(error).and_then(|result| result) {
                Ok(sources) => self.sources = Some(sources),
                Err(error) => {
                    self.source_error = Some(error.clone());
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    async fn source_input(
        &mut self,
        run: &RunPlan,
        plan: &HoloscanPlan,
        overlays: &OverlaySet,
    ) -> Result<SourcesInput, String> {
        let application = &plan
            .selection
            .as_ref()
            .ok_or("Holoscan selection is required")?
            .application;
        let mappings = views::mappings(overlays, run, plan)?;
        let mut controlled = views::controlled_mounts(run, plan)?;
        for (source, mapping) in controlled.iter_mut().zip(&mappings) {
            // Canonical mappings were resolved before launch; teardown never resolves caller aliases.
            source.source = utf8(&mapping.original)?.into();
        }
        if let Some(first) = controlled.first_mut() {
            for actual in &self.observed_mounts {
                if !actual.read_only
                    && tokio::fs::canonicalize(&actual.source).await.ok().as_ref()
                        == Some(&mappings[0].effective)
                {
                    first.target = actual.target.clone();
                    break;
                }
            }
        }
        let native = self
            .inspection
            .as_ref()
            .and_then(|info| info.mounts.as_ref())
            .ok_or("native mount evidence is unavailable")?;
        let mut volumes = Vec::new();
        for mount in native {
            let target = mount
                .destination
                .as_deref()
                .ok_or("native mount destination is unavailable")?;
            let kind = mount
                .typ
                .as_ref()
                .ok_or("native mount kind is unavailable")?
                .to_string();
            let read_only = !mount.rw.ok_or("native mount access is unavailable")?;
            if kind == "volume" {
                volumes.push(VolumeMount {
                    name: mount
                        .name
                        .clone()
                        .ok_or("native volume identity is unavailable")?,
                    consumer: views::consumer(application),
                    target: target.into(),
                    read_only,
                });
            } else if kind != "bind" {
                self.coverage
                    .push(format!("Untracked native {kind} mount at {target}"));
            } else {
                let source = mount
                    .source
                    .as_deref()
                    .ok_or("native bind source is unavailable")?;
                let canonical = tokio::fs::canonicalize(source).await.ok();
                let controlled = mappings.iter().enumerate().any(|(index, mapping)| {
                    canonical.as_ref() == Some(&mapping.effective)
                        && (index == 0 || mapping.target == target)
                });
                if !controlled {
                    self.coverage
                        .push(format!("Untracked native bind {source} -> {target}"));
                }
            }
        }
        let excluded = super::report::excluded(run);
        self.merge_system_sources(SourcesInput {
            overlays: true,
            passthrough_folders: run.passthrough_folders.clone(),
            excluded,
            bind_mounts: controlled,
            metadata_sources: docker_engine::filesystem::volume_sources(volumes),
            artifacts: PathBuf::from(&run.artifacts),
        })
    }

    /// Append the retained canonical system descriptor to native facts before one materialization.
    pub fn merge_system_sources(&self, mut native: SourcesInput) -> Result<SourcesInput, String> {
        if let Some(system) = &self.system_sources {
            if system.artifacts != native.artifacts || system.overlays != native.overlays {
                return Err("system/native source preparation identity differs".into());
            }
            native.bind_mounts.extend(system.bind_mounts.clone());
            native
                .metadata_sources
                .extend(system.metadata_sources.clone());
            native.excluded.extend(system.excluded.clone());
        }
        Ok(native)
    }
}

impl Live {
    pub async fn observe_end(&mut self, run: &RunPlan) -> Result<(), String> {
        let id = self
            .container
            .clone()
            .ok_or("owned application identity is unavailable")?;
        let mut changes = self
            .events
            .as_ref()
            .ok_or("event observation is unavailable")?
            .changes();
        loop {
            changes.borrow_and_update().clone()?;
            let info = self
                .engine
                .client()
                .inspect_container(&id, None)
                .await
                .map_err(error)?;
            container::verify_ownership(&info, run)?;
            let state = info
                .state
                .as_ref()
                .ok_or("native application state is unavailable")?;
            let running = state
                .running
                .ok_or("native application running state is unavailable")?;
            let restarting = state.restarting.unwrap_or(false);
            if !running && !restarting {
                let completed = state.pid == Some(0)
                    && state
                        .status
                        .as_ref()
                        .is_some_and(|state| state.to_string() == "exited");
                let exit = state.exit_code;
                self.inspection = Some(info);
                return match (completed, exit) {
                    (true, Some(0)) => Ok(()),
                    (true, Some(exit)) => {
                        Err(format!("native application exited with code {exit}"))
                    }
                    _ => Err("native application stopped without verified exit evidence".into()),
                };
            }
            self.inspection = Some(info);
            let child = self
                .child
                .as_mut()
                .ok_or("retained CLI child is unavailable")?;
            let output_failure = self
                .output
                .as_ref()
                .ok_or("CLI output observation is unavailable")?
                .failure();
            tokio::select! {
                status = child.wait(), if self.cli_status.is_none() => {
                    self.cli_status = Some(status.map_err(error)?);
                    self.notes.push("Native CLI exit was observed before application completion; Engine observation continued".into());
                }
                changed = changes.changed() => { changed.map_err(error)?; }
                error = output_failure => return Err(format!("native CLI output: {error}")),
            }
        }
    }

    /// Bound all helper/container stop operations by the caller's one stop deadline.
    pub async fn stop_runtime(
        &mut self,
        run: &RunPlan,
        deadline: tokio::time::Instant,
    ) -> Result<(), String> {
        tokio::time::timeout_at(deadline, async {
            if self.cli_status.is_none() && let Some(child) = &mut self.child {
                    self.cli_status = child.try_wait().map_err(error)?;
                    if self.cli_status.is_none() {
                        child.start_kill().map_err(error)?;
                        self.cli_status = Some(child.wait().await.map_err(error)?);
                    }
            }
            if self.helper.is_none() {
                self.helper = HelperProcess::recover_repository(Path::new(&run.repository_root), Duration::from_secs(run.stop_seconds))?;
            }
            if let Some(helper) = &self.helper { helper.quiesce().await?; }
            if self.attempted && self.container.is_none() {
                return Err("native CLI launched without establishing owned container identity; writer state remains unknown".into());
            }
            if let Some(id) = &self.container {
                match self.engine.client().inspect_container(id, None).await {
                    Ok(info) => container::verify_ownership(&info, run)?,
                    Err(bollard::errors::Error::DockerResponseServerError { status_code: 404, .. }) => {
                        self.notes.push("Native container is absent; final exit evidence remains unavailable".into());
                        return Ok(());
                    }
                    Err(source) => return Err(error(source)),
                }
                let seconds = i32::try_from(remaining_seconds(deadline)?).map_err(error)?;
                match self.engine.client().stop_container(id, Some(bollard::query_parameters::StopContainerOptions { t: Some(seconds), ..Default::default() })).await {
                    Ok(()) => {},
                    Err(bollard::errors::Error::DockerResponseServerError { status_code: 304 | 404, .. }) => {},
                    Err(source) => return Err(error(source)),
                }
                match self.engine.client().inspect_container(id, None).await {
                    Ok(info) => {
                        container::verify_ownership(&info, run)?;
                        let state = info.state.as_ref().ok_or("native stopped-container state is unavailable")?;
                        if state.pid != Some(0) || state.running != Some(false) {
                            return Err("native application still has live or unverified writers".into());
                        }
                        self.inspection = Some(info);
                    },
                    Err(bollard::errors::Error::DockerResponseServerError { status_code: 404, .. }) => {
                        self.notes.push("Native container is absent; final exit evidence remains unavailable".into());
                    }
                    Err(source) => return Err(error(source)),
                }
            }
            Ok(())
        }).await.map_err(|_| "Holoscan helper/container stop deadline expired".to_owned())?
    }
}

pub(super) fn remaining_seconds(deadline: tokio::time::Instant) -> Result<u64, String> {
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    if remaining.is_zero() {
        Err("Holoscan stop deadline expired".into())
    } else {
        Ok(remaining.as_secs())
    }
}

/// Launch and validate native Run while the caller-owned live handle retains every attempted resource.
pub(super) struct LaunchApplication {
    pub live: Rc<Mutex<Live>>,
}
impl Step for LaunchApplication {
    const ID: Option<&'static str> = Some("holoscan.launch-application");
    const DISPLAY_NAME: Option<&'static str> = Some("Launch Holoscan application");
    type Input = (RunPlan, HoloscanPlan, OverlaySet, String);
    type Output = String;
    async fn run(
        &mut self,
        (run, plan, views, image): Self::Input,
    ) -> orchestrator::Result<String> {
        self.live
            .lock()
            .await
            .launch(run, plan, views, image)
            .await
            .map_err(Error::message)
    }
}
async fn read_cid(path: &Path) -> Result<Option<String>, String> {
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if !metadata.is_file() || metadata.len() > CONTAINER_ID_BYTES as u64 + 2 {
        return Err(
            "native cidfile must be a regular file containing one full container ID".into(),
        );
    }
    let value = tokio::fs::read_to_string(path).await.map_err(error)?;
    let value = value.trim();
    if value.len() < CONTAINER_ID_BYTES {
        return Ok(None);
    }
    if value.len() != CONTAINER_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("native cidfile must contain one full lowercase container ID".into());
    }
    Ok(Some(value.into()))
}
fn utf8(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| "native launch path must be UTF-8".into())
}
fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
