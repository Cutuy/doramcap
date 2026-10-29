//! Compose lifecycle policy with supporting-app steps and verified teardown.
//!
//! This concrete workflow selects startup/shutdown order and recording policy.
//! Individual Compose lifecycle steps can also be used independently.
use super::system_apps::{
    self, FinalizeSystemApps, LaunchSystemApps, PrepareSystemApps, SystemAppsConfig,
    WaitForSystemApps,
};
use crate::compose::filesystem::{
    BaselineInput, CaptureFilesystemBaseline, CollectFilesystemChanges, CollectInput,
    FilesystemChanges,
};
use crate::compose::overlay::{
    FinalizeOverlays, OverlayPolicy, OverlayResults, PrepareInput, PrepareOverlays,
};
use crate::{
    compose::adapter::{RUN_LABEL, private_write, validate_name},
    compose::error::invalid,
    compose::*,
};
use futures_util::{StreamExt, future::join_all};
use orchestrator::{Input, LocalExecutor, Observer};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    future::Future,
    path::Path,
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::AsyncWriteExt,
    sync::{oneshot, watch},
};

/// Single-session runner with concurrent independent Compose operations.
///
/// `run` accepts fully prepared settings and a cooperative stop future. It never
/// discovers launch inputs or chooses profiles. Ordinary failures await
/// independent teardown. Dropping/aborting `run` cannot await cleanup and leaves
/// this manager blocked; callers must signal stop and continue awaiting the result.
/// Unknown/failed cleanup blocks subsequent runs on this manager. Direct-mount
/// recovery is available through [`super::recovery::recover_project`]. Admission
/// across processes remains the caller’s responsibility.
pub struct ComposeRunner {
    adapter: Rc<DockerAdapter>,
    blocked: bool,
}
impl ComposeRunner {
    /// Create an unblocked manager sharing the caller-owned Docker adapter.
    pub fn new(adapter: Rc<DockerAdapter>) -> Self {
        Self {
            adapter,
            blocked: false,
        }
    }
    /// Return whether an unfinished run or unverified teardown prevents admission.
    pub fn blocked(&self) -> bool {
        self.blocked
    }

    /// Execute one prepared request through startup, observation and teardown.
    ///
    /// # Parameters
    ///
    /// `spec` owns launch settings and a new artifact path. `stop` cooperatively
    /// ends observation or interrupts startup. `observer` is owned by this run's
    /// executor and receives synchronous step events, including JSON payloads
    /// which may contain sensitive configuration.
    ///
    /// # Return value and errors
    ///
    /// Returns a [`ComposeRunReport`] containing runtime failures and independent cleanup
    /// outcomes. Input/preflight errors return `Err` before launch. Filesystem or
    /// evidence-writing failures may also return `Err` later in the run. Callers
    /// must inspect report success and manager admission state separately.
    ///
    /// # Ownership and cancellation
    ///
    /// Exclusive access serializes calls on this manager. There is no admission
    /// lock across managers or processes. Ordinary runtime failures await teardown.
    /// Dropping the future cannot await cleanup and can leave resources alive and
    /// this manager blocked. Signal `stop` and await completion for orderly shutdown.
    pub async fn run(
        &mut self,
        spec: ComposeRun,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
    ) -> Result<ComposeRunReport> {
        self.run_with_ready(spec, stop, observer, None).await
    }

    /// Run a session and notify the caller after every project becomes ready.
    /// A closed receiver at notification triggers teardown. Continue awaiting
    /// this future for cleanup; plans remain valid only during the run.
    pub async fn run_with_ready(
        &mut self,
        spec: ComposeRun,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
        ready: Option<oneshot::Sender<Vec<ResolvedComposeProject>>>,
    ) -> Result<ComposeRunReport> {
        if self.blocked {
            return Err(invalid(
                "manager blocked by an unfinished or unverified experiment",
            ));
        }
        validate(&spec)?;
        let artifacts = spec.artifacts.clone();
        let mut builder = tokio::fs::DirBuilder::new();
        builder.mode(0o700);
        builder.create(&artifacts).await?;
        let artifacts = tokio::fs::canonicalize(artifacts).await?;
        snapshot(&artifacts, "before").await?;
        let diagnostics = Rc::new(RefCell::new(Vec::new()));
        let executor = Rc::new_cyclic(|weak| {
            let mut executor = LocalExecutor::new(observer)
                .with_label(&spec.id)
                .capture_payloads(true);
            super::steps::register(&mut executor, self.adapter.clone(), diagnostics.clone());
            let adapter = self.adapter.clone();
            executor.register::<PrepareOverlays>(move || PrepareOverlays {
                adapter: adapter.clone(),
            });
            executor.register::<FinalizeOverlays>(|| FinalizeOverlays);
            executor.register::<PrepareSystemApps>(|| PrepareSystemApps);
            executor.register::<WaitForSystemApps>(|| WaitForSystemApps);
            executor.register::<FinalizeSystemApps>(|| FinalizeSystemApps);
            let weak = weak.clone();
            executor.register::<LaunchSystemApps>(move || LaunchSystemApps {
                executor: weak.clone(),
            });
            executor
        });
        if spec.filesystem.is_some() {
            self.adapter.verify_overlay_namespace().await?;
        }
        let system_apps = executor
            .run::<PrepareSystemApps>(system_apps::PrepareInput {
                config: spec.system_apps.clone(),
                artifacts: artifacts.clone(),
            })
            .await
            .map_err(|e| invalid(e.to_string()))?;
        let system_count = system_apps.value().projects.len();
        let projects: Vec<_> = system_apps
            .value()
            .projects
            .iter()
            .cloned()
            .chain(spec.first_party.clone())
            .chain(spec.projects.clone())
            .collect();
        let mut keys = HashSet::new();
        if projects.iter().any(|project| !keys.insert(&project.key)) {
            return Err(invalid("system and workload project keys must be unique"));
        }
        // Resolution has no launch side effects. Independent projects share this task.
        let resolved = join_all(projects.into_iter().map(|project| {
            executor.run::<ResolveCompose>(
                Input::new(ResolveInput {
                    run: spec.id.clone(),
                    artifacts: artifacts.clone(),
                    project,
                })
                .after(&system_apps),
            )
        }))
        .await;
        let mut plans = Vec::new();
        for plan in resolved {
            plans.push(plan.map_err(|e| invalid(e.to_string()))?);
        }
        let mut input = Input::new(BaselineInput {
            projects: plans.iter().map(|p| p.value().clone()).collect(),
            artifacts: artifacts.clone(),
        });
        for plan in &plans {
            input = input.after(plan);
        }
        let baseline = executor
            .run::<CaptureFilesystemBaseline>(input)
            .await
            .map_err(|e| invalid(e.to_string()))?;
        if let Some(directory) = &spec.recovery_anchor_dir {
            super::recovery::write_anchor(
                directory,
                &artifacts,
                &spec.id,
                &plans
                    .iter()
                    .map(|plan| plan.value().clone())
                    .collect::<Vec<_>>(),
                &baseline.value().recovery_files(),
            )
            .await?;
        }
        self.blocked = true;
        let overlays = if let Some(policy) = &spec.filesystem {
            let mut preparation = Input::new(PrepareInput {
                artifacts: artifacts.clone(),
                projects: plans.iter().map(|p| p.value().clone()).collect(),
                policy: policy.clone(),
            })
            .after(&baseline);
            for plan in &plans {
                preparation = preparation.after(plan);
            }
            Some(
                executor
                    .run::<PrepareOverlays>(preparation)
                    .await
                    .map_err(|e| invalid(e.to_string()))?,
            )
        } else {
            None
        };
        let plans: Vec<_> = overlays
            .as_ref()
            .map(|o| o.value().projects.clone())
            .unwrap_or_else(|| plans.iter().map(|p| p.value().clone()).collect());
        let (finish, finished) = watch::channel(false);
        let (fault_tx, mut fault) = watch::channel(None::<String>);
        let docker = self.adapter.docker.clone();
        let run = spec.id.clone();
        let events_path = artifacts.join("docker-events.jsonl");
        let event_task = tokio::spawn(async move {
            let result = record_events(docker, run, &events_path, finished).await;
            if let Err(e) = &result {
                let _ = fault_tx.send(Some(e.to_string()));
            }
            result
        });
        let mut event_task = AbortOnDrop(event_task);
        let mut attempted_groups = Vec::new();
        tokio::pin!(stop);
        let execute = async {
            let mut groups = Vec::new();
            if system_count != 0 {
                groups.push(plans[..system_count].to_vec());
            }
            // Preserve the declared order of additional supporting projects.
            for plan in &plans[system_count..system_count + spec.first_party.len()] {
                groups.push(vec![plan.clone()]);
            }
            groups.push(plans[system_count + spec.first_party.len()..].to_vec());
            let mut previous = None;
            let mut workload_starts = HashMap::new();
            for (index, group) in groups.iter().enumerate() {
                attempted_groups.push(group.clone()); // retain every possible partial launch
                if index + 1 == groups.len() {
                    let started = join_all(group.iter().map(|plan| {
                        let mut input = Input::new(StartInput {
                            plan: plan.clone(),
                            readiness_seconds: spec.readiness_seconds,
                        })
                        .after(&baseline);
                        if let Some(overlays) = &overlays {
                            input = input.after(overlays);
                        }
                        if let Some(previous) = &previous {
                            input = input.after(previous);
                        }
                        executor.run::<StartCompose>(input)
                    }))
                    .await;
                    // Observe all outcomes before propagating failure; never abandon siblings.
                    for (plan, result) in group.iter().zip(started) {
                        workload_starts.insert(plan.key.clone(), result?);
                    }
                } else {
                    let recordings = if index == 0 && system_count != 0 {
                        system_apps.value().recordings.clone()
                    } else {
                        vec![]
                    };
                    let mut input = Input::new(system_apps::LaunchInput {
                        projects: group.clone(),
                        recordings,
                        readiness_seconds: spec.readiness_seconds,
                    })
                    .after(&baseline);
                    if let Some(overlays) = &overlays {
                        input = input.after(overlays);
                    }
                    if let Some(previous) = &previous {
                        input = input.after(previous);
                    }
                    previous = Some(executor.run::<LaunchSystemApps>(input).await?);
                }
            }
            if let Some(ready) = ready {
                ready
                    .send(plans.clone())
                    .map_err(|_| orchestrator::Error::message("readiness receiver dropped"))?;
            }
            loop {
                let states = join_all(plans.iter().map(|plan| {
                    let mut input = Input::new(plan.clone()).after(&baseline);
                    if let Some(started) = workload_starts.get(&plan.key) {
                        input = input.after(started);
                    } else if let Some(previous) = &previous {
                        input = input.after(previous);
                    }
                    executor.run::<InspectCompose>(input)
                }))
                .await;
                for (plan, state) in plans.iter().zip(states) {
                    if !DockerAdapter::ready(state?.value())
                        .map_err(|e| orchestrator::Error::message(e.to_string()))?
                    {
                        return Err(orchestrator::Error::message(format!(
                            "{} lost readiness or containers",
                            plan.key
                        )));
                    }
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            #[allow(unreachable_code)]
            Ok::<(), orchestrator::Error>(())
        };
        let outcome = tokio::select! {
            result = execute => result.map_err(|e| e.to_string()),
            _ = &mut stop => Ok(()),
            changed = fault.changed() => Err(if changed.is_err() { "Docker event stream ended".into() } else { fault.borrow().clone().unwrap_or_else(|| "Docker event stream ended".into()) }),
        };
        // Stop every writer before observing shared filesystem state. Preserve
        // runtime objects until evidence collection succeeds.
        let mut shutdowns = Vec::new();
        let mut stopped_dependencies = Vec::new();
        let mut all_stopped = true;
        for group in attempted_groups.iter().rev() {
            let stopped = join_all(group.iter().map(|plan| {
                executor.run::<QuiesceCompose>(
                    Input::new(StopInput {
                        plan: plan.clone(),
                        stop_seconds: spec.stop_seconds,
                    })
                    .after(&baseline),
                )
            }))
            .await;
            for (plan, stopped) in group.iter().zip(stopped) {
                match stopped {
                    Ok(s) => {
                        all_stopped &= s.value().stopped;
                        stopped_dependencies.push(s.clone());
                        shutdowns.push((plan.clone(), s.into_value().diagnostics));
                    }
                    Err(e) => {
                        all_stopped = false;
                        shutdowns.push((
                            plan.clone(),
                            vec![invalid(e.to_string()).diagnostic("quiesce")],
                        ));
                    }
                }
            }
        }
        let mut collect_input = Input::new(CollectInput {
            baseline: baseline.value().clone(),
            host_views: overlays
                .as_ref()
                .into_iter()
                .flat_map(|o| &o.value().trees)
                .map(|t| (t.source.clone(), t.merged.clone()))
                .collect(),
            projects: attempted_groups.iter().flatten().cloned().collect(),
        })
        .after(&baseline);
        for stopped in &stopped_dependencies {
            collect_input = collect_input.after(stopped);
        }
        let mut finalize_input =
            Input::new(system_apps.value().recordings.clone()).after(&system_apps);
        for stopped in &stopped_dependencies {
            finalize_input = finalize_input.after(stopped);
        }
        // Writers are stopped; read-only recording checks and metadata collection are independent.
        let (recording, filesystem_output) =
            tokio::join!(executor.run::<FinalizeSystemApps>(finalize_input), async {
                if !all_stopped {
                    return None;
                }
                match executor
                    .run::<CollectFilesystemChanges>(collect_input)
                    .await
                {
                    Ok(changes) => Some(changes),
                    Err(e) => {
                        diagnostics
                            .borrow_mut()
                            .push(invalid(e.to_string()).diagnostic("filesystem changes"));
                        None
                    }
                }
            },);
        if let Err(error) = &recording {
            diagnostics
                .borrow_mut()
                .push(invalid(error.to_string()).diagnostic("finalize recording"));
        }
        let writers_stopped = all_stopped;
        let filesystem_changes = filesystem_output.as_ref().map(|o| o.value().clone());
        if overlays.is_none()
            && let Some(changes) = &filesystem_changes
            && let Err(error) =
                super::compose_lifecycle::save_outer_evidence(&artifacts, &spec.id, changes)
        {
            diagnostics
                .borrow_mut()
                .push(invalid(error).diagnostic("filesystem checkpoint"));
            all_stopped = false; // Retain resources until their evidence is durable.
        }
        let overlay_output =
            if let (Some(changes), Some(overlays)) = (&filesystem_output, &overlays) {
                match executor
                    .run::<FinalizeOverlays>(
                        Input::new(overlays.value().clone())
                            .after(overlays)
                            .after(changes),
                    )
                    .await
                {
                    Ok(result) => Some(result),
                    Err(e) => {
                        diagnostics
                            .borrow_mut()
                            .push(invalid(e.to_string()).diagnostic("finalize overlays"));
                        None
                    }
                }
            } else {
                None
            };
        // Hosted recovery anchors require retaining experiment volumes whenever
        // their metadata coverage is incomplete. Standalone runs keep their
        // existing caller-selected volume policy.
        let preserve_volumes = spec.recovery_anchor_dir.is_some()
            && filesystem_changes
                .as_ref()
                .is_some_and(|changes| !changes.named_volumes.coverage_complete);
        let cleanup = join_all(shutdowns.into_iter().map(|(plan, stop_diagnostics)| {
            let executor = &executor;
            let baseline = &baseline;
            let filesystem_output = &filesystem_output;
            let overlay_output = &overlay_output;
            let overlays = &overlays;
            async move {
                let mut result = if all_stopped
                    && (overlays.is_none() || overlay_output.is_some())
                    && let Some(changes) = &filesystem_output
                {
                    let mut removal_plan = plan.clone();
                    if preserve_volumes {
                        removal_plan.remove_volumes = false;
                    }
                    let mut input = Input::new(StopInput {
                        plan: removal_plan,
                        stop_seconds: spec.stop_seconds,
                    })
                    .after(baseline)
                    .after(changes);
                    if let Some(finalized) = &overlay_output {
                        input = input.after(finalized);
                    }
                    match executor.run::<RemoveCompose>(input).await {
                        Ok(c) => c.into_value(),
                        Err(e) => Cleanup {
                            project: plan.name.clone(),
                            diagnostics: vec![invalid(e.to_string()).diagnostic("remove")],
                            remaining: vec!["unknown".into()],
                            retained_volumes: vec![],
                            verified: false,
                        },
                    }
                } else {
                    Cleanup {
                        project: plan.name.clone(),
                        diagnostics: vec![],
                        remaining: vec![
                            "preserved: shutdown or filesystem evidence incomplete".into(),
                        ],
                        retained_volumes: vec![],
                        verified: false,
                    }
                };
                result.diagnostics.extend(stop_diagnostics);
                result
            }
        }))
        .await;
        let _ = finish.send(true);
        let event_result = (&mut event_task.0).await;
        let event_stream_healthy = matches!(event_result, Ok(Ok(())));
        if !event_stream_healthy {
            diagnostics.borrow_mut().push(
                invalid(format!("event capture: {event_result:?}")).diagnostic("Docker events"),
            );
        }
        if executor.observer_failures() != 0 {
            diagnostics
                .borrow_mut()
                .push(invalid("workflow observer lost events").diagnostic("observer"));
        }
        if preserve_volumes {
            diagnostics.borrow_mut().push(
                invalid("named-volume evidence incomplete; volumes retained")
                    .diagnostic("filesystem changes"),
            );
        }
        if let Err(e) = snapshot(&artifacts, "after").await {
            diagnostics.borrow_mut().push(e.diagnostic("post-state"));
        }
        let boundary_clean = all_stopped
            && filesystem_output.is_some()
            && (overlays.is_none() || overlay_output.is_some())
            && cleanup.iter().all(|x| {
                x.verified
                    && x.diagnostics.is_empty()
                    && (!preserve_volumes || x.retained_volumes.is_empty())
            });
        let report = ComposeRunReport {
            id: spec.id,
            execution_error: outcome.err(),
            diagnostics: diagnostics.borrow().clone(),
            cleanup,
            boundary_clean,
            writers_stopped,
            recording_finalized: recording.is_ok(),
            event_stream_healthy,
            filesystem_changes,
            filesystem_results: overlay_output.map(|o| o.into_value()),
        };
        private_write(
            &artifacts.join("report.json"),
            &serde_json::to_vec_pretty(&report)?,
        )
        .await?;
        self.blocked = !boundary_clean;
        Ok(report)
    }
}
struct AbortOnDrop<T>(tokio::task::JoinHandle<T>);
impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}
fn validate(spec: &ComposeRun) -> Result<()> {
    validate_name(&spec.id)?;
    if !spec.artifacts.is_absolute()
        || spec.projects.is_empty()
        || spec.readiness_seconds == 0
        || !(1..=300).contains(&spec.stop_seconds)
    {
        return Err(invalid(
            "require absolute new artifact directory, foreign projects, positive readiness and stop_seconds 1..300",
        ));
    }
    spec.system_apps.validate()?;
    if spec
        .recovery_anchor_dir
        .as_ref()
        .is_some_and(|path| !path.is_absolute())
    {
        return Err(invalid("recovery anchor directory must be absolute"));
    }
    let mut keys = HashSet::new();
    for project in spec.first_party.iter().chain(&spec.projects) {
        validate_name(&project.key)?;
        if !project.directory.is_absolute() || !keys.insert(project.key.clone()) {
            return Err(invalid(
                "require absolute project directories and unique keys; system is reserved",
            ));
        }
    }
    Ok(())
}

async fn snapshot(root: &Path, phase: &str) -> Result<()> {
    let mut state = BTreeMap::new();
    for name in ["meminfo", "loadavg", "uptime", "sys/kernel/random/boot_id"] {
        state.insert(
            name,
            tokio::fs::read_to_string(format!("/proc/{name}")).await?,
        );
    }
    private_write(
        &root.join(format!("machine-{phase}.json")),
        &serde_json::to_vec_pretty(&state)?,
    )
    .await
}
async fn record_events(
    docker: bollard::Docker,
    run: String,
    path: &Path,
    mut finish: watch::Receiver<bool>,
) -> Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .await?;
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| invalid(e.to_string()))?
        .as_secs()
        .to_string();
    let stream = docker.events(Some(bollard::query_parameters::EventsOptions {
        since: Some(since),
        filters: Some(HashMap::from([(
            "label".into(),
            vec![format!("{RUN_LABEL}={run}")],
        )])),
        ..Default::default()
    }));
    tokio::pin!(stream);
    let mut draining = false;
    let quiet = tokio::time::sleep(Duration::from_millis(200));
    let deadline = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(quiet, deadline);
    loop {
        tokio::select! {
            changed = finish.changed(), if !draining => {
                if changed.is_err() || *finish.borrow() {
                    draining = true;
                    quiet.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(200));
                    deadline.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(5));
                }
            }
            event = stream.next() => {
                let event = event.ok_or_else(|| invalid("Docker events stream ended unexpectedly"))?.map_err(|e| crate::compose::error::engine("Docker events", e))?;
                let mut bytes = serde_json::to_vec(&event)?; bytes.push(b'\n');
                file.write_all(&bytes).await?;
                if draining { quiet.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(200)); }
            }
            _ = &mut quiet, if draining => { file.flush().await?; return Ok(()); }
            _ = &mut deadline, if draining => { return Err(invalid("Docker event stream did not become quiet after teardown")); }
        }
    }
}

/// Inputs to the Compose workflow: workloads and supporting apps sharing one
/// run lifetime. Images must exist locally.
/// Artifact directory must be new and have an existing parent.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComposeRun {
    /// Some isolates writable directory binds through OverlayFS. None uses the
    /// supplied mounts directly. Omitted JSON retains the default write-back policy.
    #[serde(default = "default_filesystem")]
    pub filesystem: Option<OverlayPolicy>,
    /// Caller-selected run identifier used in Docker ownership labels and project names.
    pub id: String,
    /// Absolute, new artifact directory; its parent must already exist.
    pub artifacts: PathBuf,
    /// Existing private directory outside every workload mount, used to freeze
    /// recovery identities before launch. The caller protects it from workloads.
    #[serde(default)]
    pub recovery_anchor_dir: Option<PathBuf>,
    /// Foreign Compose projects owned by this run; at least one is required.
    pub projects: Vec<ComposeProject>,
    /// Additional prepared first-party projects, such as a configured DORA/ROS
    /// subscriber. Started before foreign projects and stopped after them.
    #[serde(default)]
    pub first_party: Vec<ComposeProject>,
    /// Supporting-app configuration, interpreted by the dedicated system-app steps.
    #[serde(default)]
    pub system_apps: SystemAppsConfig,
    /// Positive startup/readiness budget in seconds; default 60.
    #[serde(default = "readiness_default")]
    pub readiness_seconds: u64,
    /// Graceful stop budget per project in seconds, from 1 through 300; default 15.
    #[serde(default = "stop_default")]
    pub stop_seconds: u64,
}
fn default_filesystem() -> Option<OverlayPolicy> {
    Some(OverlayPolicy::default())
}
fn readiness_default() -> u64 {
    60
}
fn stop_default() -> u64 {
    15
}

/// Outcome and cleanup are separate. Inspect `successful()` and `boundary_clean`
/// before treating evidence as complete or admitting another experiment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComposeRunReport {
    /// Successful overlay disposition, absent on preparation/harvest failure.
    #[serde(default)]
    pub filesystem_results: Option<OverlayResults>,
    /// Run identifier.
    pub id: String,
    /// Startup/observation failure, independently of cleanup results.
    pub execution_error: Option<String>,
    /// Structured workflow and step failure evidence.
    pub diagnostics: Vec<Diagnostic>,
    /// Independent teardown results for attempted projects.
    pub cleanup: Vec<Cleanup>,
    /// Whether owned runtime resources were verified cleaned up.
    pub boundary_clean: bool,
    /// All attempted project writers stopped, independently of evidence or removal.
    #[serde(default)]
    pub writers_stopped: bool,
    /// Whether configured supporting-app recordings passed finalization checks.
    pub recording_finalized: bool,
    /// Whether event capture completed without a detected error; not a losslessness guarantee.
    pub event_stream_healthy: bool,
    /// Category-specific metadata manifests. Expected changes/coverage warnings
    /// are informational; unavailable required collection prevents removal.
    #[serde(default)]
    pub filesystem_changes: Option<FilesystemChanges>,
}
impl ComposeRunReport {
    pub fn successful(&self) -> bool {
        self.execution_error.is_none()
            && self.boundary_clean
            && self.recording_finalized
            && self.event_stream_healthy
            && self.diagnostics.is_empty()
            && self.cleanup.iter().all(|x| x.diagnostics.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filesystem_policy_default_and_direct_mount_opt_out() {
        // Arrange
        let mut config = serde_json::json!({"id":"run", "artifacts":"/tmp/example", "projects":[{
            "key":"workload", "directory":"/tmp", "files":["compose.yaml"]
        }]});
        // Act / Assert
        let defaulted: ComposeRun = serde_json::from_value(config.clone()).unwrap();
        assert!(defaulted.filesystem.is_some());
        assert!(validate(&defaulted).is_ok());
        config["filesystem"] = serde_json::Value::Null;
        let direct: ComposeRun = serde_json::from_value(config).unwrap();
        assert!(direct.filesystem.is_none());
        assert!(validate(&direct).is_ok());
    }
}
