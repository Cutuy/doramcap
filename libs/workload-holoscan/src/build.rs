//! Admitted native application Build with durable latest-attempt and streaming output ownership.
use crate::{
    BuildOutcome, BuildProgress, BuildRecord, BuildRequest, BuildResult, Selection, build_result,
    builds::Attempt, cli_options, discovery::normalize_language, environment::configure_container,
    process::HelperProcess, routing::verify_native_daemon,
};
use compose_apps::{cgroup::Role, compose::adapter::repository_scope, shared_dora::owner};
use docker_engine::{ARTIFACTS_LABEL, CONTROLLER_LABEL, Engine, RUN_LABEL};
use orchestrator::{Error, Result, Step};
use std::{os::unix::process::ExitStatusExt, path::Path, process::Stdio, time::Duration};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Child,
    sync::watch,
};
use workload_lifecycle::{
    admission::Admission,
    launch::{HostShmAccess, RunPlan},
};

const CIDFILE: &str = "builder.cid";
const BUILD_COMPONENT: &str = "build";
const LOG_BUFFER_BYTES: usize = 64 * 1024;

/// Build one native application in Docker, retaining admission through output/cleanup/publication.
/// The caller owns this future and must signal shutdown then await it; dropping is not async cleanup.
/// Native routing/options/logging stay native; only Console-owned Docker identity/placement is added.
///
/// # Evidence
/// Latest incomplete state precedes daemon checks. Only a complete successful latest attempt supplies
/// usable Build evidence. The retained builder's Engine image is observed before container removal;
/// this artifact does not predict the image of a later native Run.
///
/// # Output and cancellation
/// Two fixed processing buffers stream unbounded output to private per-attempt disk logs. Disk/read
/// failures cancel the CLI and remain separate from process/image/cleanup facts. Shutdown kills/reaps
/// the actual direct Child before scope cleanup; an owner must keep its runtime alive until return.
pub struct BuildApplication {
    engine: Engine,
    shutdown: watch::Receiver<bool>,
    progress: watch::Sender<BuildProgress>,
}
impl BuildApplication {
    /// Inject the configured Engine, owner stop/close signal and latest disk-log counters.
    pub fn new(
        engine: Engine,
        shutdown: watch::Receiver<bool>,
        progress: watch::Sender<BuildProgress>,
    ) -> Self {
        Self {
            engine,
            shutdown,
            progress,
        }
    }
}
impl Step for BuildApplication {
    const ID: Option<&'static str> = Some("holoscan.build");
    const DISPLAY_NAME: Option<&'static str> = Some("Build Holoscan application");
    type Input = BuildRequest;
    type Output = BuildOutcome;

    async fn run(&mut self, request: BuildRequest) -> Result<BuildOutcome> {
        let attempt_request = request
            .attempt
            .ok_or_else(|| invalid("Build attempt is required"))?;
        let key = attempt_request
            .store
            .as_ref()
            .and_then(|store| store.key.as_ref())
            .ok_or_else(|| invalid("Build store and key are required"))?;
        let selection = key
            .selection
            .clone()
            .ok_or_else(|| invalid("Build selection is required"))?;
        if selection.application.is_empty()
            || !Path::new(&request.cli).is_absolute()
            || [
                &request.cli,
                &selection.application,
                &selection.mode,
                &selection.language,
            ]
            .iter()
            .any(|value| value.contains('\0'))
        {
            return Err(invalid(
                "Build requires an absolute CLI and nonempty application, with no NUL in native arguments",
            ));
        }
        if request.readiness_seconds == 0
            || request.stop_seconds == 0
            || request.stop_seconds > i32::MAX as u64
        {
            return Err(invalid(
                "Build requires positive readiness/stop budgets and stop_seconds within i32",
            ));
        }
        if request.host_shm_access != HostShmAccess::Passthrough as i32 {
            return Err(invalid(
                "Holoscan Build requires explicit HOST_SHM_ACCESS_PASSTHROUGH; native Docker controls cannot prove BLOCKED",
            ));
        }
        let admission = Admission::acquire(Path::new(&key.repository_root))
            .map_err(|error| invalid(&error.to_string()))?;
        let mut attempt = Attempt::begin_admitted(attempt_request, admission).await?;
        let run = RunPlan {
            id: attempt.state().id.clone(),
            repository_root: utf8(attempt.repository())?,
            artifacts: utf8(attempt.artifacts())?,
            readiness_seconds: request.readiness_seconds,
            stop_seconds: request.stop_seconds,
            host_shm_access: request.host_shm_access,
            ..Default::default()
        };
        self.progress.send_replace(BuildProgress {
            attempt_id: run.id.clone(),
            ..Default::default()
        });
        let mut result = BuildResult::default();
        let operation = async {
            cli_options::validate(&run, &attempt.artifacts().join(CIDFILE)).map_err(|error| invalid(&error))?;
            tokio::select! {
                biased;
                _ = cancelled(&mut self.shutdown) => { result.cancelled = true; return Ok(()); }
                observed = self.engine.negotiate() => observed.map_err(|error| invalid(&error.to_string()))?,
            }
            tokio::select! {
                biased;
                _ = cancelled(&mut self.shutdown) => { result.cancelled = true; return Ok(()); }
                observed = verify_native_daemon(attempt.repository(), &self.engine) => { observed.map_err(|error| invalid(&error.to_string()))?; }
            }
            self.reset(&run).await.map_err(|error| { result.cleanup_error = error; invalid("Prior Build/Run cleanup failed") })?;
            result.cleanup_verified = true;
            if stopping(&self.shutdown) { result.cancelled = true; return Ok(()); }
            let stdout = match log_file(&attempt.stdout_path()).await {
                Ok(file) => file,
                Err(error) => { result.output_error = error.to_string(); return Ok(()); }
            };
            let stderr = match log_file(&attempt.stderr_path()).await {
                Ok(file) => file,
                Err(error) => { result.output_error = error.to_string(); return Ok(()); }
            };
            result.cleanup_verified = false;
            let helper = match HelperProcess::prepare(&self.engine, &run, Role::User, BUILD_COMPONENT).await {
                Ok(helper) => helper,
                Err(error) => {
                    result.process_error = error;
                    match self.reset(&run).await {
                        Ok(()) => result.cleanup_verified = true,
                        Err(error) => result.cleanup_error = error,
                    }
                    return Ok(());
                }
            };
            let cidfile = attempt.artifacts().join(CIDFILE);
            let launched = async {
                let options = cli_options::owned(&run, helper.cgroup_parent(), &cidfile).map_err(|error| invalid(&error))?;
                let mut command = helper.command(Path::new(&request.cli)).map_err(|error| invalid(&error))?;
                command.args(native_args(&selection, &options)).current_dir(attempt.repository())
                    .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
                configure_container(&mut command);
                if stopping(&self.shutdown) { result.cancelled = true; return Ok(()); }
                let mut child = command.spawn().map_err(|error| Error::Io { operation: "spawn Holoscan Build", source: error })?;
                self.wait_child(&mut child, stdout, stderr, &helper, Duration::from_secs(run.stop_seconds), &mut result).await;
                Ok::<(), Error>(())
            }.await;
            if let Err(error) = launched { append(&mut result.process_error, error.to_string()); }
            // The actual Child has been reaped before any helper stop, preventing late bootstrap registration.
            let helper_stopped = helper.quiesce().await;
            if let Err(error) = &helper_stopped { append(&mut result.cleanup_error, error.clone()); }
            let scope = repository_scope(attempt.repository());
            let scope = utf8(&scope)?;
            let reservation = owner(attempt.repository());
            let resources = self.engine.resources(&scope, &reservation);
            let containers_stopped = resources.quiesce(run.stop_seconds).await;
            if let Err(error) = &containers_stopped { append(&mut result.cleanup_error, error.to_string()); }
            if result.termination == Some(build_result::Termination::ExitCode(0)) && !result.cancelled {
                match self.builder_image(&cidfile, &run, &scope).await {
                    Ok(image) => {
                        result.observed_image_id = image.clone();
                        result.record = Some(BuildRecord { selection: Some(selection), image_id: image, source: attempt.state().source.clone() });
                    }
                    Err(error) => append(&mut result.image_error, error),
                }
            }
            if containers_stopped.is_ok() {
                if let Err(error) = resources.remove(None).await { append(&mut result.cleanup_error, error.to_string()); }
                else if helper_stopped.is_ok() {
                    match helper.remove().await {
                        Ok(()) => result.cleanup_verified = true,
                        Err(error) => append(&mut result.cleanup_error, error),
                    }
                }
            }
            Ok::<(), Error>(())
        }.await;
        let error = operation
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
        if !error.is_empty() {
            append(&mut result.process_error, error.clone());
        }
        let candidate = result.clone();
        let (attempt, publication_error) = match attempt.finish(result).await {
            Ok(saved) => (saved, String::new()),
            Err(error) => {
                let mut state = attempt.state().clone();
                // The caller receives invocation facts even if durable publication was not acknowledged.
                if state.result.is_none() {
                    state.result = Some(candidate);
                }
                (state, error.to_string())
            }
        };
        Ok(BuildOutcome {
            attempt: Some(attempt),
            error,
            publication_error,
        })
    }
}
impl BuildApplication {
    async fn reset(&self, run: &RunPlan) -> std::result::Result<(), String> {
        let repository = Path::new(&run.repository_root);
        let helper =
            HelperProcess::recover_repository(repository, Duration::from_secs(run.stop_seconds))?;
        let helper_stopped = match &helper {
            Some(helper) => helper.quiesce().await,
            None => Ok(()),
        };
        let scope = repository_scope(repository);
        let scope = scope
            .to_str()
            .ok_or("Holoscan controller scope must be UTF-8")?;
        let reservation = owner(repository);
        let resources = self.engine.resources(scope, &reservation);
        let containers_stopped = resources
            .quiesce(run.stop_seconds)
            .await
            .map_err(|error| error.to_string());
        helper_stopped?;
        containers_stopped?;
        resources
            .remove(None)
            .await
            .map_err(|error| error.to_string())?;
        if let Some(helper) = helper {
            helper.remove().await?;
        }
        Ok(())
    }

    async fn wait_child(
        &mut self,
        child: &mut Child,
        stdout: File,
        stderr: File,
        helper: &HelperProcess,
        stop: Duration,
        result: &mut BuildResult,
    ) {
        let out = child.stdout.take().expect("piped Build stdout");
        let err = child.stderr.take().expect("piped Build stderr");
        let mut out = Box::pin(pump(out, stdout, self.progress.clone(), true));
        let mut err = Box::pin(pump(err, stderr, self.progress.clone(), false));
        let mut out_done = false;
        let mut err_done = false;
        let mut kill = false;
        loop {
            tokio::select! {
                biased;
                _ = cancelled(&mut self.shutdown) => { result.cancelled = true; kill = true; break; }
                status = child.wait() => {
                    termination(status, result);
                    break;
                }
                outcome = &mut out, if !out_done => {
                    out_done = true;
                    if let Err(error) = outcome { append(&mut result.output_error, error); kill = true; break; }
                }
                outcome = &mut err, if !err_done => {
                    err_done = true;
                    if let Err(error) = outcome { append(&mut result.output_error, error); kill = true; break; }
                }
            }
        }
        if kill {
            if let Err(error) = child.start_kill() {
                append(
                    &mut result.process_error,
                    format!("kill Build CLI: {error}"),
                );
            }
            termination(child.wait().await, result);
        }
        // Descendants can retain pipe descriptors after the CLI exits; stop the owned tree before EOF.
        if let Err(error) = helper.quiesce().await {
            append(&mut result.cleanup_error, error);
        }
        let drain = async {
            if !out_done && let Err(error) = out.await {
                append(&mut result.output_error, error);
            }
            if !err_done && let Err(error) = err.await {
                append(&mut result.output_error, error);
            }
        };
        if tokio::time::timeout(stop, drain).await.is_err() {
            append(
                &mut result.output_error,
                "Build output did not close within the stop budget".into(),
            );
        }
    }

    async fn builder_image(
        &self,
        cidfile: &Path,
        run: &RunPlan,
        scope: &str,
    ) -> std::result::Result<String, String> {
        let id = fs::read_to_string(cidfile).await.map_err(|error| {
            format!(
                "read retained builder cidfile {}: {error}",
                cidfile.display()
            )
        })?;
        let id = id.trim();
        if id.len() != 64 || !id.bytes().all(|value| value.is_ascii_hexdigit()) {
            return Err("Retained builder cidfile is not a full Docker container ID".into());
        }
        let container = self
            .engine
            .client()
            .inspect_container(id, None)
            .await
            .map_err(|error| format!("inspect retained native builder: {error}"))?;
        let labels = container
            .config
            .as_ref()
            .and_then(|config| config.labels.as_ref())
            .ok_or("Retained builder ownership labels are absent")?;
        for (label, expected) in [
            (CONTROLLER_LABEL, scope),
            (RUN_LABEL, run.id.as_str()),
            (ARTIFACTS_LABEL, run.artifacts.as_str()),
        ] {
            if labels.get(label).map(String::as_str) != Some(expected) {
                return Err(format!("Retained builder {label} ownership differs"));
            }
        }
        let state = container
            .state
            .as_ref()
            .ok_or("Retained builder state is absent")?;
        if state.running != Some(false) || state.pid != Some(0) {
            return Err("Retained builder still has live or unverified writers".into());
        }
        container
            .image
            .filter(|image| !image.is_empty())
            .ok_or("Retained builder image identity is absent".into())
    }
}

fn native_args(selection: &Selection, options: &str) -> Vec<String> {
    let mut args = vec![
        "build".into(),
        "--persistent".into(),
        format!("--docker-opts={options}"),
    ];
    let language = normalize_language(&selection.language);
    if !language.is_empty() {
        args.extend(["--language".into(), language]);
    }
    args.extend(["--".into(), selection.application.clone()]);
    if !selection.mode.is_empty() {
        args.push(selection.mode.clone());
    }
    args
}
async fn log_file(path: &Path) -> Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .await
        .map_err(|error| Error::Io {
            operation: "create Holoscan Build log",
            source: error,
        })
}
async fn pump(
    mut source: impl AsyncRead + Unpin,
    mut destination: File,
    progress: watch::Sender<BuildProgress>,
    stdout: bool,
) -> std::result::Result<(), String> {
    let mut buffer = vec![0; LOG_BUFFER_BYTES];
    loop {
        let count = source
            .read(&mut buffer)
            .await
            .map_err(|error| format!("read Build output: {error}"))?;
        if count == 0 {
            break;
        }
        destination
            .write_all(&buffer[..count])
            .await
            .map_err(|error| format!("write Build log: {error}"))?;
        progress.send_modify(|value| {
            let bytes = if stdout {
                &mut value.stdout_bytes
            } else {
                &mut value.stderr_bytes
            };
            *bytes = bytes.saturating_add(count as u64);
        });
    }
    destination
        .sync_all()
        .await
        .map_err(|error| format!("sync Build log: {error}"))
}
fn termination(status: std::io::Result<std::process::ExitStatus>, result: &mut BuildResult) {
    match status {
        Ok(status) => {
            result.termination = status
                .code()
                .map(build_result::Termination::ExitCode)
                .or_else(|| status.signal().map(build_result::Termination::Signal))
        }
        Err(error) => append(
            &mut result.process_error,
            format!("wait Build CLI: {error}"),
        ),
    }
}
fn stopping(shutdown: &watch::Receiver<bool>) -> bool {
    *shutdown.borrow() || shutdown.has_changed().is_err()
}
async fn cancelled(shutdown: &mut watch::Receiver<bool>) {
    loop {
        if stopping(shutdown) {
            return;
        }
        if shutdown.changed().await.is_err() {
            return;
        }
    }
}
fn append(target: &mut String, error: String) {
    if !target.is_empty() {
        target.push('\n');
    }
    target.push_str(&error);
}
fn invalid(message: &str) -> Error {
    Error::Configuration(message.into())
}
fn utf8(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid("Holoscan Build paths must be UTF-8"))
}
