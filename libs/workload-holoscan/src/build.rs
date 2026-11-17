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
    time::Instant,
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
    socket: String,
    shutdown: watch::Receiver<bool>,
    progress: watch::Sender<BuildProgress>,
}
impl BuildApplication {
    /// Inject the configured Engine socket, owner stop/close signal and latest disk-log counters.
    pub fn new(
        socket: String,
        shutdown: watch::Receiver<bool>,
        progress: watch::Sender<BuildProgress>,
    ) -> Self {
        Self {
            socket,
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
            || Instant::now()
                .checked_add(Duration::from_secs(request.readiness_seconds))
                .is_none()
            || request.stop_seconds == 0
            || request.stop_seconds > i32::MAX as u64
        {
            return Err(invalid(
                "Build requires positive supported-clock readiness/stop budgets and stop_seconds within i32",
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
            logs: Some(attempt.logs()),
            ..Default::default()
        });
        let mut result = BuildResult::default();
        let operation = async {
            cli_options::validate(&run, &attempt.artifacts().join(CIDFILE)).map_err(|error| invalid(&error))?;
            if stopping(&self.shutdown) { result.cancelled = true; return Ok(()); }
            let mut engine = Engine::new(self.socket.clone(), run.readiness_seconds).map_err(|error| invalid(&error.to_string()))?;
            let readiness = Instant::now() + Duration::from_secs(run.readiness_seconds);
            tokio::select! {
                biased;
                _ = cancelled(&mut self.shutdown) => { result.cancelled = true; return Ok(()); }
                observed = within(readiness, "verify Build daemon", async {
                    engine.negotiate().await.map_err(|error| error.to_string())?;
                    verify_native_daemon(attempt.repository(), &engine).await.map(|_| ()).map_err(|error| error.to_string())
                }) => observed.map_err(|error| invalid(&error))?,
            }
            crate::reset::reset_project(&engine, &run).await.map_err(|error| { result.cleanup_error = error; invalid("Prior Build/Run cleanup failed") })?;
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
            let helper = match within(Instant::now() + Duration::from_secs(run.readiness_seconds), "prepare Build helper", HelperProcess::prepare(&engine, &run, Role::User, BUILD_COMPONENT)).await {
                Ok(helper) => helper,
                Err(error) => {
                    result.process_error = error;
                    match crate::reset::reset_project(&engine, &run).await {
                        Ok(()) => result.cleanup_verified = true,
                        Err(error) => result.cleanup_error = error,
                    }
                    return Ok(());
                }
            };
            let cidfile = attempt.artifacts().join(CIDFILE);
            let mut child_reaped = true;
            let mut cleanup_deadline = None;
            let mut stopped_helper = None;
            let launched = async {
                let options = cli_options::owned(&run, helper.cgroup_parent(), &cidfile).map_err(|error| invalid(&error))?;
                let mut command = helper.command(Path::new(&request.cli)).map_err(|error| invalid(&error))?;
                command.args(native_args(&selection, &options, attempt.repository())).current_dir(attempt.repository())
                    .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
                configure_container(&mut command);
                if stopping(&self.shutdown) { result.cancelled = true; return Ok(()); }
                let mut child = command.spawn().map_err(|error| Error::Io { operation: "spawn Holoscan Build", source: error })?;
                let out = pump(child.stdout.take().expect("piped Build stdout"), stdout, self.progress.clone(), true);
                let err = pump(child.stderr.take().expect("piped Build stderr"), stderr, self.progress.clone(), false);
                tokio::pin!(out, err);
                let (reaped, out_done, err_done, deadline) = self.wait_child(&mut child, out.as_mut(), err.as_mut(), Duration::from_secs(run.stop_seconds), &mut result).await;
                child_reaped = reaped;
                cleanup_deadline = Some(deadline);
                if !reaped { return Ok(()); }
                // Observe natural builder completion before helper signals or Engine stop can change it.
                let scope = utf8(&repository_scope(attempt.repository()))?;
                if result.termination == Some(build_result::Termination::ExitCode(0)) && !result.cancelled {
                    match within(deadline, "inspect retained builder", Self::builder_image(&engine, &cidfile, &run, &scope, helper.cgroup_parent())).await {
                        Ok(image) => {
                            result.observed_image_id = image.clone();
                            result.record = Some(BuildRecord { selection: Some(selection.clone()), image_id: image, source: attempt.state().source.clone() });
                        }
                        Err(error) => append(&mut result.image_error, error),
                    }
                }

                stopped_helper = Some(within(deadline, "stop Build helper before output EOF", helper.quiesce()).await);
                let drain = async {
                    if !out_done && let Err(error) = out.await { append(&mut result.output_error, error); }
                    if !err_done && let Err(error) = err.await { append(&mut result.output_error, error); }
                };
                if tokio::time::timeout_at(deadline, drain).await.is_err() {
                    append(&mut result.output_error, "Build output did not close within the stop budget".into());
                }
                Ok::<(), Error>(())
            }.await;
            if let Err(error) = launched { append(&mut result.process_error, error.to_string()); }
            if !child_reaped {
                append(&mut result.cleanup_error, "Direct Build Child reaping is unverified; helper ownership is retained for recovery".into());
                return Ok(());
            }
            let deadline = cleanup_deadline.unwrap_or_else(|| Instant::now() + Duration::from_secs(run.stop_seconds));
            // The actual Child has been reaped before helper stop, preventing late bootstrap registration.
            let helper_stopped = match stopped_helper {
                Some(observed) => observed,
                None => within(deadline, "stop Build helper", helper.quiesce()).await,
            };
            if let Err(error) = &helper_stopped { append(&mut result.cleanup_error, error.clone()); }
            let scope = repository_scope(attempt.repository());
            let scope = utf8(&scope)?;
            let reservation = owner(attempt.repository());
            let resources = engine.resources(&scope, &reservation);
            let containers_stopped = within(deadline, "stop Build containers", async { resources.quiesce(deadline.saturating_duration_since(Instant::now()).as_secs()).await.map_err(|error| error.to_string()) }).await;
            if let Err(error) = &containers_stopped { append(&mut result.cleanup_error, error.to_string()); }
            if containers_stopped.is_ok() {
                if let Err(error) = within(deadline, "remove Build containers", async { resources.remove(None).await.map_err(|error| error.to_string()) }).await { append(&mut result.cleanup_error, error); }
                else if helper_stopped.is_ok() {
                    match within(deadline, "remove Build helper", helper.remove()).await {
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
        Ok(finish_build(&mut attempt, result, error).await)
    }
}
impl BuildApplication {
    async fn wait_child(
        &mut self,
        child: &mut Child,
        mut out: std::pin::Pin<
            &mut impl std::future::Future<Output = std::result::Result<(), String>>,
        >,
        mut err: std::pin::Pin<
            &mut impl std::future::Future<Output = std::result::Result<(), String>>,
        >,
        stop: Duration,
        result: &mut BuildResult,
    ) -> (bool, bool, bool, Instant) {
        let mut out_done = false;
        let mut err_done = false;
        let kill = loop {
            tokio::select! {
                biased;
                _ = cancelled(&mut self.shutdown) => { result.cancelled = true; break true; }
                status = child.wait() => {
                    let kill = status.is_err();
                    termination(status, result);
                    break kill;
                }
                outcome = &mut out, if !out_done => {
                    out_done = true;
                    if let Err(error) = outcome { append(&mut result.output_error, error); break true; }
                }
                outcome = &mut err, if !err_done => {
                    err_done = true;
                    if let Err(error) = outcome { append(&mut result.output_error, error); break true; }
                }
            }
        };
        let deadline = Instant::now() + stop;
        let mut reaped = !kill;
        if kill {
            if let Err(error) = child.start_kill() {
                append(
                    &mut result.process_error,
                    format!("kill Build CLI: {error}"),
                );
            }
            match tokio::time::timeout_at(deadline, child.wait()).await {
                Ok(status) => {
                    reaped = status.is_ok();
                    termination(status, result);
                }
                Err(_) => append(
                    &mut result.process_error,
                    "Build CLI did not reap within the stop budget".into(),
                ),
            }
        }
        (reaped, out_done, err_done, deadline)
    }

    async fn builder_image(
        engine: &Engine,
        cidfile: &Path,
        run: &RunPlan,
        scope: &str,
        cgroup_parent: &str,
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
        let container = engine
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
        if container
            .host_config
            .as_ref()
            .and_then(|host| host.cgroup_parent.as_deref())
            != Some(cgroup_parent)
        {
            return Err(
                "Retained builder cgroup placement differs from the owned application parent"
                    .into(),
            );
        }
        let state = container
            .state
            .as_ref()
            .ok_or("Retained builder state is absent")?;
        if state.running != Some(false) || state.pid != Some(0) {
            return Err("Retained builder still has live or unverified writers".into());
        }
        if state.status.as_ref().map(AsRef::<str>::as_ref) != Some("exited")
            || !crate::container_started(state.started_at.as_deref())
        {
            return Err("Retained builder has no observed started/exited completion".into());
        }
        if state.exit_code != Some(0) {
            return Err(format!(
                "Retained native builder exit is {:?}; successful Build requires exit 0",
                state.exit_code
            ));
        }
        container
            .image
            .filter(|image| !image.is_empty())
            .ok_or("Retained builder image identity is absent".into())
    }
}

/// Settle caller-owned log entries before terminal store publication; facts remain independent.
async fn finish_build(
    attempt: &mut Attempt,
    mut result: BuildResult,
    error: String,
) -> BuildOutcome {
    if !error.is_empty() {
        append(&mut result.process_error, error.clone());
    }
    if let Err(error) = sync_output_directory(attempt.artifacts()).await {
        append(&mut result.output_error, error);
    }
    let candidate = result.clone();
    let logs = Some(attempt.logs());
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
    BuildOutcome {
        attempt: Some(attempt),
        error,
        publication_error,
        logs,
    }
}
#[cfg(test)]
thread_local! {
    static FAIL_OUTPUT_DIRECTORY_ACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
async fn sync_output_directory(path: &Path) -> std::result::Result<(), String> {
    let directory = File::open(path)
        .await
        .map_err(|error| format!("open Build output directory {}: {error}", path.display()))?;
    if !directory
        .metadata()
        .await
        .map_err(|error| format!("inspect Build output directory: {error}"))?
        .is_dir()
    {
        return Err("Build output location is no longer a directory".into());
    }
    #[cfg(test)]
    if FAIL_OUTPUT_DIRECTORY_ACK.with(std::cell::Cell::get) {
        return Err("injected Build output directory acknowledgement failure".into());
    }
    directory
        .sync_all()
        .await
        .map_err(|error| format!("sync Build output directory {}: {error}", path.display()))
}

fn native_args(selection: &Selection, options: &str, repository: &Path) -> Vec<String> {
    let mut args = vec![
        "--project-root".into(),
        repository.to_string_lossy().into_owned(),
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
        destination
            .flush()
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
async fn within<T>(
    deadline: Instant,
    operation: &str,
    future: impl std::future::Future<Output = std::result::Result<T, String>>,
) -> std::result::Result<T, String> {
    tokio::time::timeout_at(deadline, future)
        .await
        .map_err(|_| format!("{operation}: deadline exceeded"))?
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BuildAttemptRequest, BuildKey, BuildStoreRequest};
    use std::os::unix::fs::PermissionsExt;

    fn directory(label: &str) -> std::path::PathBuf {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "holoscan-build-{label}-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    #[tokio::test]
    async fn full_logs_stream_to_disk_and_progress_retains_references() {
        // Arrange: output exceeds each fixed processing buffer several times.
        let root = directory("output");
        let data: Vec<u8> = (0..LOG_BUFFER_BYTES * 3 + 17)
            .map(|i| (i % 251) as u8)
            .collect();
        let destination = log_file(&root.join("stdout.log")).await.unwrap();
        let (progress, observed) = watch::channel(BuildProgress::default());
        let (mut writer, reader) = tokio::io::duplex(LOG_BUFFER_BYTES);
        // Act
        let ((), saved) = tokio::join!(
            async {
                writer.write_all(&data).await.unwrap();
                drop(writer);
            },
            pump(reader, destination, progress, true)
        );
        // Assert
        saved.unwrap();
        assert_eq!(fs::read(root.join("stdout.log")).await.unwrap(), data);
        assert_eq!(observed.borrow().stdout_bytes, data.len() as u64);
        assert_eq!(observed.borrow().stderr_bytes, 0);
        fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn failed_output_is_explicit_without_false_written_bytes() {
        // Arrange
        let destination = OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .await
            .unwrap();
        let (progress, observed) = watch::channel(BuildProgress::default());
        // Act
        let error = pump(&b"required output"[..], destination, progress, true)
            .await
            .unwrap_err();
        // Assert
        assert!(error.contains("write Build log"));
        assert_eq!(observed.borrow().stdout_bytes, 0);
    }

    #[tokio::test]
    async fn failed_or_cancelled_daemon_check_replaces_prior_success() {
        if crate::test_support::ran_in_isolated_process() {
            return;
        }
        // Arrange
        let root = directory("daemon");
        let selection = Selection {
            application: "app".into(),
            ..Default::default()
        };
        let store = BuildStoreRequest {
            root: root.join("builds").to_str().unwrap().into(),
            key: Some(BuildKey {
                repository_root: root.to_str().unwrap().into(),
                selection: Some(selection.clone()),
            }),
        };
        let mut previous = Attempt::begin(BuildAttemptRequest {
            store: Some(store.clone()),
            id: "previous".into(),
            ..Default::default()
        })
        .await
        .unwrap();
        previous
            .finish(BuildResult {
                termination: Some(build_result::Termination::ExitCode(0)),
                cleanup_verified: true,
                record: Some(BuildRecord {
                    selection: Some(selection),
                    image_id: "sha256:previous".into(),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .await
            .unwrap();
        for (id, cancelled) in [("daemon-failed", false), ("cancelled", true)] {
            let (_shutdown, receiver) = watch::channel(cancelled);
            let (progress, _) = watch::channel(BuildProgress::default());
            let mut build = BuildApplication::new(
                root.join("missing.sock").to_str().unwrap().into(),
                receiver,
                progress,
            );
            // Act
            let outcome = build
                .run(BuildRequest {
                    attempt: Some(BuildAttemptRequest {
                        store: Some(store.clone()),
                        id: id.into(),
                        ..Default::default()
                    }),
                    cli: "/missing/cli".into(),
                    readiness_seconds: 1,
                    stop_seconds: 1,
                    host_shm_access: HostShmAccess::Passthrough as i32,
                })
                .await
                .unwrap();
            let latest = crate::builds::lookup(store.clone()).await.unwrap();
            // Assert
            assert_eq!(latest.latest.as_ref().unwrap().id, id);
            assert!(latest.record.is_none());
            assert_eq!(
                latest
                    .latest
                    .as_ref()
                    .unwrap()
                    .result
                    .as_ref()
                    .unwrap()
                    .cancelled,
                cancelled
            );
            assert!(cancelled || !outcome.error.is_empty());
            assert_eq!(latest.logs, outcome.logs);
            assert_eq!(
                latest.logs.as_ref().unwrap().stdout,
                outcome.logs.as_ref().unwrap().stdout
            );
        }
        fs::remove_dir_all(root).await.unwrap();
    }
    #[tokio::test]
    async fn builder_evidence_requires_natural_started_exit_and_owned_placement() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        // Arrange: native Engine observations include values also present on never-started containers.
        for (status, running, pid, exit, started, parent, eligible) in [
            (
                "exited",
                false,
                0,
                0,
                Some("2026-11-17T01:00:00Z"),
                "app.slice",
                true,
            ),
            (
                "running",
                true,
                23,
                0,
                Some("2026-11-17T01:00:00Z"),
                "app.slice",
                false,
            ),
            (
                "created",
                false,
                0,
                0,
                Some("0001-01-01T00:00:00Z"),
                "app.slice",
                false,
            ),
            ("exited", false, 0, 0, None, "app.slice", false),
            (
                "exited",
                false,
                0,
                0,
                Some("0001-01-01T00:00:00Z"),
                "app.slice",
                false,
            ),
            (
                "exited",
                false,
                0,
                7,
                Some("2026-11-17T01:00:00Z"),
                "app.slice",
                false,
            ),
            (
                "exited",
                false,
                0,
                0,
                Some("2026-11-17T01:00:00Z"),
                "other.slice",
                false,
            ),
        ] {
            let root = directory("builder-state");
            let cidfile = root.join(CIDFILE);
            let id = "a".repeat(64);
            fs::write(&cidfile, &id).await.unwrap();
            let socket = root.join("engine.sock");
            let listener = tokio::net::UnixListener::bind(&socket).unwrap();
            let engine = Engine::new(socket.to_str().unwrap(), 1).unwrap();
            let run = RunPlan {
                id: "build".into(),
                repository_root: utf8(&root).unwrap(),
                artifacts: utf8(&root).unwrap(),
                ..Default::default()
            };
            let scope = utf8(&repository_scope(&root)).unwrap();
            let labels = std::collections::HashMap::from([
                (CONTROLLER_LABEL, scope.as_str()),
                (RUN_LABEL, run.id.as_str()),
                (ARTIFACTS_LABEL, run.artifacts.as_str()),
            ]);
            let body = serde_json::json!({"Id":id,"Image":"sha256:built","Config":{"Labels":labels},"HostConfig":{"CgroupParent":parent},"State":{"Status":status,"Running":running,"Pid":pid,"ExitCode":exit,"StartedAt":started}}).to_string();
            let server = async {
                let (mut connection, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 1024];
                while !bytes.ends_with(b"\r\n\r\n") {
                    let count = connection.read(&mut buffer).await.unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                }
                assert!(
                    String::from_utf8(bytes)
                        .unwrap()
                        .contains(&format!("/containers/{id}/json"))
                );
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                connection.write_all(response.as_bytes()).await.unwrap();
            };
            // Act
            let (observed, ()) = tokio::join!(
                BuildApplication::builder_image(&engine, &cidfile, &run, &scope, "app.slice"),
                server
            );
            // Assert
            assert_eq!(
                observed.is_ok(),
                eligible,
                "{status} start={started:?} exit={exit} parent={parent}: {observed:?}"
            );
            if eligible {
                assert_eq!(observed.unwrap(), "sha256:built");
            }
            fs::remove_dir_all(root).await.unwrap();
        }
    }
    #[tokio::test]
    async fn output_directory_ack_failure_preserves_facts_but_denies_usable_evidence() {
        if crate::test_support::ran_in_isolated_process() {
            return;
        }
        // Arrange: file bytes are synced, while their directory acknowledgement fails separately.
        let root = directory("output-ack");
        let selection = Selection {
            application: "sample".into(),
            ..Default::default()
        };
        let store = BuildStoreRequest {
            root: utf8(&root.join("builds")).unwrap(),
            key: Some(BuildKey {
                repository_root: utf8(&root).unwrap(),
                selection: Some(selection.clone()),
            }),
        };
        let mut attempt = Attempt::begin(BuildAttemptRequest {
            store: Some(store.clone()),
            id: "completed".into(),
            source: None,
        })
        .await
        .unwrap();
        let stdout = log_file(&attempt.stdout_path()).await.unwrap();
        pump(
            &b"saved Build output"[..],
            stdout,
            watch::channel(BuildProgress::default()).0,
            true,
        )
        .await
        .unwrap();
        let result = BuildResult {
            termination: Some(build_result::Termination::ExitCode(0)),
            cleanup_verified: true,
            observed_image_id: "sha256:built".into(),
            record: Some(BuildRecord {
                selection: Some(selection),
                image_id: "sha256:built".into(),
                source: None,
            }),
            ..Default::default()
        };
        FAIL_OUTPUT_DIRECTORY_ACK.with(|value| value.set(true));
        // Act
        let outcome = finish_build(&mut attempt, result, String::new()).await;
        FAIL_OUTPUT_DIRECTORY_ACK.with(|value| value.set(false));
        // Assert: output durability alone fails; known process/image/cleanup observations survive.
        let result = outcome.attempt.as_ref().unwrap().result.as_ref().unwrap();
        assert_eq!(
            result.termination,
            Some(build_result::Termination::ExitCode(0))
        );
        assert!(result.cleanup_verified);
        assert_eq!(result.observed_image_id, "sha256:built");
        assert!(
            result
                .output_error
                .contains("directory acknowledgement failure")
        );
        assert!(result.record.is_none());
        assert!(outcome.publication_error.is_empty());
        assert_eq!(
            fs::read(outcome.logs.unwrap().stdout).await.unwrap(),
            b"saved Build output"
        );
        let current = crate::builds::lookup(store).await.unwrap();
        assert!(current.record.is_none());
        assert_eq!(
            current.latest.unwrap().result.unwrap().output_error,
            result.output_error
        );
        fs::remove_dir_all(root).await.unwrap();
    }
}
