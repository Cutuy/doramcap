//! Record one local Compose run with fresh CPU monitoring and verified teardown.
//!
//! This concrete workflow selects startup/shutdown order and recording policy.
//! Individual Compose lifecycle steps can also be used independently.
use crate::compose::filesystem::{
    BaselineInput, CaptureFilesystemBaseline, CollectFilesystemChanges, CollectInput,
    FilesystemChanges,
};
use crate::{
    compose::adapter::{RUN_LABEL, private_write, validate_name},
    compose::error::invalid,
    compose::*,
};
use futures_util::StreamExt;
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
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::watch,
};

/// Serial runner for the recorded Compose workflow, with an injected Docker adapter.
///
/// `run` accepts fully prepared settings and a cooperative stop future. It never
/// discovers launch inputs or chooses profiles. Ordinary failures await
/// independent teardown. Dropping/aborting `run` cannot await cleanup and leaves
/// this manager blocked; callers must signal stop and continue awaiting the result.
/// Unknown/failed cleanup blocks subsequent runs on this manager. No crash recovery
/// or cross-process admission lock is implemented; external Engine access is cooperative.
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
        let mut executor = LocalExecutor::new(observer)
            .with_label(&spec.id)
            .capture_payloads(true);
        macro_rules! register {
            ($s:ident) => {{
                let adapter = self.adapter.clone();
                let diagnostics = diagnostics.clone();
                executor.register::<$s>(move || $s {
                    adapter: adapter.clone(),
                    diagnostics: diagnostics.clone(),
                });
            }};
        }
        register!(ResolveCompose);
        register!(StartCompose);
        register!(InspectCompose);
        register!(QuiesceCompose);
        register!(RemoveCompose);
        let adapter = self.adapter.clone();
        executor.register::<CaptureFilesystemBaseline>(move || CaptureFilesystemBaseline {
            adapter: adapter.clone(),
        });
        let adapter = self.adapter.clone();
        executor.register::<CollectFilesystemChanges>(move || CollectFilesystemChanges {
            adapter: adapter.clone(),
        });
        // Resolve all projects before the first workload is launched.
        let mut plans = Vec::new();
        let system_file = artifacts.join("system-services.yaml");
        private_write(
            &system_file,
            include_bytes!("../../compose/system-services.yaml"),
        )
        .await?;
        let system = ComposeProject {
            key: "system".into(),
            directory: artifacts.clone(),
            files: vec![system_file],
            env_files: vec![],
            profiles: vec![],
            services: vec![],
            completed_services: vec![],
            remove_volumes: false,
            environment: BTreeMap::from([
                ("DORAMCAP_MONITOR_IMAGE".into(), spec.monitor.image.clone()),
                (
                    "DORAMCAP_MONITOR_BACKEND".into(),
                    spec.monitor.backend.clone(),
                ),
                (
                    "DORAMCAP_ARTIFACTS".into(),
                    artifacts.to_string_lossy().into_owned(),
                ),
            ]),
        };
        for project in std::iter::once(system)
            .chain(spec.first_party.clone())
            .chain(spec.projects.clone())
        {
            let plan = executor
                .run::<ResolveCompose>(ResolveInput {
                    run: spec.id.clone(),
                    artifacts: artifacts.clone(),
                    project,
                })
                .await
                .map_err(|e| invalid(e.to_string()))?;
            plans.push(plan);
        }
        let baseline = executor
            .run::<CaptureFilesystemBaseline>(BaselineInput {
                projects: plans.iter().map(|p| p.value().clone()).collect(),
                artifacts: artifacts.clone(),
            })
            .await
            .map_err(|e| invalid(e.to_string()))?;
        self.blocked = true;
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
        let mut attempted = Vec::new();
        tokio::pin!(stop);
        let execute = async {
            for plan in &plans {
                attempted.push(plan.clone()); // retain even if Compose creates only some resources
                executor
                    .run::<StartCompose>(
                        Input::new(StartInput {
                            plan: plan.value().clone(),
                            readiness_seconds: spec.readiness_seconds,
                        })
                        .after(plan),
                    )
                    .await?;
                if plan.value().key == "system" {
                    monitor_ready(&artifacts, spec.readiness_seconds)
                        .await
                        .map_err(|e| orchestrator::Error::message(e.to_string()))?;
                }
            }
            loop {
                for plan in &plans {
                    let state = executor.run::<InspectCompose>(plan.clone()).await?;
                    if !DockerAdapter::ready(state.value())
                        .map_err(|e| orchestrator::Error::message(e.to_string()))?
                    {
                        return Err(orchestrator::Error::message(format!(
                            "{} lost readiness or containers",
                            plan.value().key
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
        for plan in attempted.iter().rev() {
            let stopped = executor
                .run::<QuiesceCompose>(
                    Input::new(StopInput {
                        plan: plan.value().clone(),
                        stop_seconds: spec.stop_seconds,
                    })
                    .after(plan),
                )
                .await;
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
        let mut collect_input = Input::new(CollectInput {
            baseline: baseline.value().clone(),
            projects: attempted.iter().map(|p| p.value().clone()).collect(),
        })
        .after(&baseline);
        for stopped in &stopped_dependencies {
            collect_input = collect_input.after(stopped);
        }
        let filesystem_output = if all_stopped {
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
        } else {
            None
        };
        let filesystem_changes = filesystem_output.as_ref().map(|o| o.value().clone());
        let mut cleanup = Vec::new();
        for (plan, stop_diagnostics) in shutdowns {
            let mut result = if all_stopped && filesystem_changes.is_some() {
                match executor
                    .run::<RemoveCompose>(
                        Input::new(StopInput {
                            plan: plan.value().clone(),
                            stop_seconds: spec.stop_seconds,
                        })
                        .after(&plan)
                        .after(filesystem_output.as_ref().expect("collection succeeded")),
                    )
                    .await
                {
                    Ok(c) => c.into_value(),
                    Err(e) => Cleanup {
                        project: plan.value().name.clone(),
                        diagnostics: vec![invalid(e.to_string()).diagnostic("remove")],
                        remaining: vec!["unknown".into()],
                        retained_volumes: vec![],
                        verified: false,
                    },
                }
            } else {
                Cleanup {
                    project: plan.value().name.clone(),
                    diagnostics: vec![],
                    remaining: vec!["preserved: shutdown or filesystem evidence incomplete".into()],
                    retained_volumes: vec![],
                    verified: false,
                }
            };
            result.diagnostics.extend(stop_diagnostics);
            cleanup.push(result);
        }
        // Producer has stopped; check a bounded terminal MCAP footer and stable size.
        let recording = finalized_recording(&artifacts.join("cpu.mcap")).await;
        if let Err(e) = &recording {
            diagnostics
                .borrow_mut()
                .push(e.diagnostic("finalize recording"));
        }
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
        if let Err(e) = snapshot(&artifacts, "after").await {
            diagnostics.borrow_mut().push(e.diagnostic("post-state"));
        }
        let boundary_clean = cleanup
            .iter()
            .all(|x| x.verified && x.diagnostics.is_empty());
        let report = ComposeRunReport {
            id: spec.id,
            execution_error: outcome.err(),
            diagnostics: diagnostics.borrow().clone(),
            cleanup,
            boundary_clean,
            recording_finalized: recording.is_ok(),
            event_stream_healthy,
            filesystem_changes,
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
        || !(1..=60).contains(&spec.stop_seconds)
    {
        return Err(invalid(
            "require absolute new artifact directory, foreign projects, positive readiness and stop_seconds 1..60",
        ));
    }
    if !spec.monitor.allow_unconfined_seccomp {
        return Err(invalid(
            "current embedded writer requires explicitly approved unconfined seccomp; no silent relaxation",
        ));
    }
    if !["bpf", "synthetic"].contains(&spec.monitor.backend.as_str()) {
        return Err(invalid("unsupported monitor backend"));
    }
    let mut keys = HashSet::from(["system".to_owned()]);
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
async fn monitor_ready(directory: &Path, timeout: u64) -> Result<()> {
    let start = tokio::time::Instant::now();
    let mut previous = None;
    loop {
        if let Ok(metadata) = tokio::fs::metadata(directory.join("cpu.pb")).await {
            if previous.is_some_and(|n| n > 0 && metadata.len() > n)
                && tokio::fs::try_exists(directory.join("cpu.mcap")).await?
            {
                return Ok(());
            }
            previous = Some(metadata.len());
        }
        if start.elapsed() > Duration::from_secs(timeout) {
            return Err(invalid(
                "CPU monitor did not advance telemetry/arm MCAP recording",
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

/// Inputs to the recorded Compose workflow: foreign projects, supporting services
/// and a fresh CPU recorder, sharing one run lifetime. Images must exist locally.
/// Artifact directory must be new and have an existing parent.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComposeRun {
    /// Caller-selected run identifier used in Docker ownership labels and project names.
    pub id: String,
    /// Absolute, new artifact directory; its parent must already exist.
    pub artifacts: PathBuf,
    /// Foreign Compose projects owned by this run; at least one is required.
    pub projects: Vec<ComposeProject>,
    /// Additional prepared first-party projects, such as a configured DORA/ROS
    /// subscriber. Started before foreign projects and stopped after them.
    #[serde(default)]
    pub first_party: Vec<ComposeProject>,
    /// Fresh CPU monitor configuration; its embedded writer records this run.
    pub monitor: CpuMonitor,
    /// Positive startup/readiness budget in seconds; default 60.
    #[serde(default = "readiness_default")]
    pub readiness_seconds: u64,
    /// Graceful stop budget per project in seconds, from 1 through 60; default 15.
    #[serde(default = "stop_default")]
    pub stop_seconds: u64,
}
fn readiness_default() -> u64 {
    60
}
fn stop_default() -> u64 {
    15
}

/// First-party collector settings. Recording uses its embedded MCAP writer.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuMonitor {
    /// Locally available monitor image reference, resolved to an immutable image ID.
    pub image: String,
    /// CpuMonitor backend: `bpf` (default) or `synthetic` for fixtures.
    #[serde(default = "native_backend")]
    pub backend: String,
    /// Explicit trusted-host exception needed by the current io_uring writer.
    /// This implementation refuses to silently relax Docker's seccomp profile.
    pub allow_unconfined_seccomp: bool,
}
fn native_backend() -> String {
    "bpf".into()
}

/// Outcome and cleanup are separate. Inspect `successful()` and `boundary_clean`
/// before treating evidence as complete or admitting another experiment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComposeRunReport {
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
    /// Whether the native CPU MCAP passed finalization checks.
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
