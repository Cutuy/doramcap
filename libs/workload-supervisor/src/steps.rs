//! Typed native phases registered on the existing orchestrator executor.
use crate::{
    MonitorRun, Ownership, Result, control, error,
    ownership::{self, Runtime},
};
use orchestrator::{LocalExecutor, Step};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    fs::{File, OpenOptions},
    io::Write,
    os::{
        fd::AsRawFd,
        unix::{
            fs::{DirBuilderExt, MetadataExt},
            process::CommandExt,
        },
    },
    path::Path,
    process::Stdio,
    rc::Rc,
    time::Duration,
};

pub(crate) type State = Rc<RefCell<Option<Runtime>>>;
fn step_error(e: impl std::fmt::Display) -> orchestrator::Error {
    orchestrator::Error::message(e.to_string())
}
const TEMPLATE: &str =
    include_str!("../../../apps/system-monitor/deploy/supervisor/recording.conf");

pub(crate) fn validate(run: &MonitorRun) -> Result<()> {
    if run.id.is_empty()
        || run.id.len() > 64
        || !run
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || run.readiness_seconds == 0
        || run.readiness_seconds > 300
        || !(1..=300).contains(&run.stop_seconds)
    {
        return Err("require run id [A-Za-z0-9_-], readiness/stop deadlines 1..300".into());
    }
    for path in [
        &run.output_directory,
        &run.supervisord,
        &run.supervisorctl,
        &run.monitor_binary,
        &run.cgroup_parent,
    ] {
        if !Path::new(path).is_absolute()
            || path
                .chars()
                .any(|c| c.is_control() || "%\"'\\;#".contains(c))
        {
            return Err("require absolute paths without control, interpolation, quoting or INI comment characters".into());
        }
    }
    if run.output_directory.len() + "/control.sock".len() >= 108 {
        return Err("Unix socket path exceeds Linux limit".into());
    }
    if Path::new(&run.output_directory).exists() {
        return Err(
            "output directory already exists; inspect ownership evidence before reuse".into(),
        );
    }
    for binary in [&run.supervisord, &run.supervisorctl, &run.monitor_binary] {
        let meta = std::fs::metadata(binary).map_err(error)?;
        use std::os::unix::fs::PermissionsExt;
        if !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
            return Err(format!("executable unavailable: {binary}"));
        }
    }
    let parent = std::fs::canonicalize(&run.cgroup_parent).map_err(error)?;
    if !parent.starts_with("/sys/fs/cgroup")
        || parent == Path::new("/sys/fs/cgroup")
        || parent.to_str() != Some(&run.cgroup_parent)
    {
        return Err("require canonical delegated cgroup v2 parent below /sys/fs/cgroup".into());
    }
    if parent.join(&run.id).exists() {
        return Err("owned cgroup exists; unfinished ownership blocks admission".into());
    }
    OpenOptions::new()
        .write(true)
        .open(parent.join("cgroup.procs"))
        .map_err(error)?;
    File::open("/sys/kernel/btf/vmlinux").map_err(error)?;
    let status = std::fs::read_to_string("/proc/self/status").map_err(error)?;
    let caps = status
        .lines()
        .find_map(|s| s.strip_prefix("CapEff:\t"))
        .and_then(|s| u64::from_str_radix(s, 16).ok())
        .unwrap_or(0);
    if caps & (1 << 21) == 0 && caps & ((1 << 38) | (1 << 39)) != ((1 << 38) | (1 << 39)) {
        return Err("BPF access requires effective CAP_BPF/CAP_PERFMON or CAP_SYS_ADMIN".into());
    }
    Ok(())
}

pub struct Prepare {
    pub(crate) state: State,
}
impl Step for Prepare {
    type Input = MonitorRun;
    type Output = Ownership;
    async fn run(&mut self, run: MonitorRun) -> orchestrator::Result<Ownership> {
        self.prepare(run).await.map_err(step_error)
    }
}
impl Prepare {
    async fn prepare(&self, run: MonitorRun) -> Result<Ownership> {
        validate(&run)?;
        let mut check = tokio::process::Command::new(&run.monitor_binary);
        let output = control::command(
            check.arg("--check-bpf").arg("--records").arg("32768"),
            run.readiness_seconds,
        )
        .await?;
        let capabilities: serde_json::Value = serde_json::from_str(&output).map_err(error)?;
        for field in [
            "bpf_compiled",
            "kernel_version_ok",
            "cgroup_v2",
            "btf_readable",
            "attachment_verified",
        ] {
            if capabilities[field] != true {
                return Err(format!("monitor preflight failed: {field}"));
            }
        }
        let dir = Path::new(&run.output_directory);
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(dir)
            .map_err(error)?;
        let config = format!(
            "[unix_http_server]\nfile={}/control.sock\nchmod=0600\n[supervisord]\nnodaemon=true\nlogfile={}/supervisord.log\nlogfile_maxbytes=0\npidfile={}/supervisord.pid\nchildlogdir={}\n[supervisorctl]\nserverurl=unix://{}/control.sock\n[rpcinterface:supervisor]\nsupervisor.rpcinterface_factory=supervisor.rpcinterface:make_main_rpcinterface\n{}",
            run.output_directory,
            run.output_directory,
            run.output_directory,
            run.output_directory,
            run.output_directory,
            TEMPLATE
        );
        ownership::private_file(&dir.join("supervisord.conf"), config.as_bytes())?;
        ownership::private_file(&dir.join("run.pb"), &prost::Message::encode_to_vec(&run))?;
        let ledger = Ownership {
            cgroup: format!("{}/{}", run.cgroup_parent, run.id),
            boot_id: std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .map_err(error)?
                .trim()
                .into(),
            monitor_sha256: hex::encode(Sha256::digest(
                std::fs::read(&run.monitor_binary).map_err(error)?,
            )),
            config_sha256: hex::encode(Sha256::digest(config.as_bytes())),
            run: Some(run),
            ..Default::default()
        };
        ownership::save(&ledger)?;
        *self.state.borrow_mut() = Some(Runtime {
            child: None,
            kill: None,
            ledger: ledger.clone(),
            forced: false,
            daemon_reaped: true,
        });
        std::fs::create_dir(&ledger.cgroup).map_err(error)?;
        let mut state = self.state.borrow_mut();
        let runtime = state.as_mut().unwrap();
        let meta = std::fs::metadata(&ledger.cgroup).map_err(error)?;
        runtime.ledger.cgroup_inode = meta.ino();
        runtime.ledger.cgroup_device = meta.dev();
        ownership::save(&runtime.ledger)?;
        runtime.kill = Some(
            OpenOptions::new()
                .write(true)
                .open(Path::new(&ledger.cgroup).join("cgroup.kill"))
                .map_err(error)?,
        );
        Ok(runtime.ledger.clone())
    }
}

pub struct Launch {
    pub(crate) state: State,
}
impl Step for Launch {
    type Input = Ownership;
    type Output = Ownership;
    async fn run(&mut self, ownership: Ownership) -> orchestrator::Result<Ownership> {
        self.launch(ownership).await.map_err(step_error)
    }
}
impl Launch {
    async fn launch(&self, ownership: Ownership) -> Result<Ownership> {
        let run = ownership.run.as_ref().ok_or("run missing")?;
        ownership::verify(&ownership)?;
        let procs = OpenOptions::new()
            .write(true)
            .open(Path::new(&ownership.cgroup).join("cgroup.procs"))
            .map_err(error)?;
        let mut command = std::process::Command::new(&run.supervisord);
        command
            .arg("-n")
            .arg("-c")
            .arg(Path::new(&run.output_directory).join("supervisord.conf"))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LC_ALL", "C")
            .env("DORAMCAP_MONITOR", &run.monitor_binary)
            .env("DORAMCAP_RUN", &run.output_directory)
            .env("DORAMCAP_STOP_SECONDS", run.stop_seconds.to_string())
            .stdin(Stdio::null())
            .stdout(
                File::create(Path::new(&run.output_directory).join("daemon.stdout.log"))
                    .map_err(error)?,
            )
            .stderr(
                File::create(Path::new(&run.output_directory).join("daemon.stderr.log"))
                    .map_err(error)?,
            );
        // Only async-signal-safe write occurs between fork and exec. Writing zero
        // moves this child before Supervisor can fork a program.
        unsafe {
            command.pre_exec(move || {
                if libc::write(procs.as_raw_fd(), b"0".as_ptr().cast(), 1) != 1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = tokio::process::Command::from(command)
            .kill_on_drop(true)
            .spawn()
            .map_err(error)?;
        let pid = child.id().ok_or("daemon PID absent")?;
        {
            let mut state = self.state.borrow_mut();
            let runtime = state.as_mut().ok_or("runtime absent")?;
            runtime.child = Some(child);
            runtime.daemon_reaped = false;
            runtime.ledger.daemon_pid = pid;
            runtime.ledger.daemon_start_ticks = ownership::start_ticks(pid)?;
            ownership::save(&runtime.ledger)?;
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(run.readiness_seconds);
        loop {
            if let Some(status) = self
                .state
                .borrow_mut()
                .as_mut()
                .unwrap()
                .child
                .as_mut()
                .unwrap()
                .try_wait()
                .map_err(error)?
            {
                return Err(format!(
                    "Supervisor exited before control readiness: {status}"
                ));
            }
            if control::ctl(run, "status", 1).await.is_ok() {
                break;
            }
            // STOPPED uses a nonzero CLI exit; socket existence permits an explicit start.
            if Path::new(&run.output_directory)
                .join("control.sock")
                .exists()
            {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("Supervisor control readiness deadline exceeded".into());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        control::ctl(run, "start", run.readiness_seconds).await?;
        let status = control::ctl(run, "status", 2).await?;
        let pid = control::monitor_pid(&status)?;
        {
            let mut state = self.state.borrow_mut();
            let runtime = state.as_mut().unwrap();
            runtime.ledger.monitor_pid = pid;
            runtime.ledger.monitor_start_ticks = ownership::start_ticks(pid)?;
            ownership::save(&runtime.ledger)?;
        }
        Ok(self.state.borrow().as_ref().unwrap().ledger.clone())
    }
}

pub struct Inspect {
    pub(crate) state: State,
}
impl Step for Inspect {
    type Input = MonitorRun;
    type Output = ();
    async fn run(&mut self, run: MonitorRun) -> orchestrator::Result<()> {
        if self
            .state
            .borrow_mut()
            .as_mut()
            .and_then(|r| r.child.as_mut())
            .ok_or_else(|| step_error("daemon handle absent"))?
            .try_wait()
            .map_err(step_error)?
            .is_some()
        {
            return Err(step_error("Supervisor exited early"));
        }
        let status = control::ctl(&run, "status", 2).await.map_err(step_error)?;
        let evidence = self.state.borrow().as_ref().unwrap().ledger.clone();
        let pid = control::monitor_pid(&status).map_err(step_error)?;
        if pid != evidence.monitor_pid
            || ownership::start_ticks(pid).map_err(step_error)? != evidence.monitor_start_ticks
        {
            return Err(step_error(
                "monitor PID changed; restart ends this recording attempt",
            ));
        }
        if !control::running(&status) {
            return Err(step_error(format!("monitor lost liveness: {status}")));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Quiesced {
    pub writers_stopped: bool,
    pub forced: bool,
    pub diagnostics: Vec<String>,
}
pub struct Quiesce {
    pub(crate) state: State,
}
impl Step for Quiesce {
    type Input = MonitorRun;
    type Output = Quiesced;
    async fn run(&mut self, run: MonitorRun) -> orchestrator::Result<Quiesced> {
        self.quiesce(&run).await.map_err(step_error)
    }
}
impl Quiesce {
    async fn quiesce(&self, run: &MonitorRun) -> Result<Quiesced> {
        let mut diagnostics = Vec::new();
        if self.state.borrow().is_none() {
            return Ok(Quiesced {
                writers_stopped: true,
                forced: false,
                diagnostics,
            });
        }
        let has_child = self.state.borrow().as_ref().unwrap().child.is_some();
        if has_child {
            if let Err(e) = control::ctl(run, "stop", run.stop_seconds + 2).await {
                diagnostics.push(e);
            }
            if let Err(e) = control::ctl(run, "shutdown", 2).await {
                diagnostics.push(e);
            }
            let mut child = self
                .state
                .borrow_mut()
                .as_mut()
                .unwrap()
                .child
                .take()
                .unwrap();
            match tokio::time::timeout(Duration::from_secs(run.stop_seconds + 2), child.wait())
                .await
            {
                Ok(result) => {
                    result.map_err(error)?;
                }
                Err(_) => {
                    diagnostics.push("daemon stop deadline exceeded".into());
                    self.state.borrow_mut().as_mut().unwrap().forced = true;
                    // The retained child handle cannot refer to an unrelated PID.
                    child.kill().await.map_err(error)?;
                }
            }
            self.state.borrow_mut().as_mut().unwrap().daemon_reaped = true;
        }
        let log = std::fs::read_to_string(Path::new(&run.output_directory).join("supervisord.log"))
            .unwrap_or_default();
        if log.contains("SIGKILL") {
            self.state.borrow_mut().as_mut().unwrap().forced = true;
            diagnostics.push("Supervisor used SIGKILL".into());
        }
        {
            let mut state = self.state.borrow_mut();
            let runtime = state.as_mut().unwrap();
            if runtime.ledger.cgroup_inode == 0 {
                return Err("cgroup creation was not verified; preserve ownership evidence".into());
            }
            if !ownership::empty(&runtime.ledger)? {
                runtime.forced = true;
                ownership::verify(&runtime.ledger)?;
                runtime
                    .kill
                    .as_mut()
                    .ok_or("cgroup kill handle absent")?
                    .write_all(b"1")
                    .map_err(error)?;
            }
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(run.stop_seconds);
        loop {
            let empty = ownership::empty(&self.state.borrow().as_ref().unwrap().ledger)?;
            if empty {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("owned cgroup remains populated".into());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let state = self.state.borrow();
        Ok(Quiesced {
            writers_stopped: true,
            forced: state.as_ref().unwrap().forced,
            diagnostics,
        })
    }
}

pub struct Remove {
    pub(crate) state: State,
}
impl Step for Remove {
    type Input = ();
    type Output = bool;
    async fn run(&mut self, _: ()) -> orchestrator::Result<bool> {
        let mut state = self.state.borrow_mut();
        let Some(runtime) = state.as_mut() else {
            return Ok(true);
        };
        if !runtime.daemon_reaped
            || runtime.child.is_some()
            || !ownership::empty(&runtime.ledger).map_err(step_error)?
        {
            return Err(step_error("daemon not reaped or cgroup populated"));
        }
        std::fs::remove_dir(&runtime.ledger.cgroup).map_err(step_error)?;
        runtime.ledger.cleanup_verified = true;
        ownership::save(&runtime.ledger).map_err(step_error)?;
        Ok(true)
    }
}

pub(crate) fn register(executor: &mut LocalExecutor, state: State) {
    macro_rules! register {
        ($kind:ident) => {{
            let state = state.clone();
            executor.register::<$kind>(move || $kind {
                state: state.clone(),
            });
        }};
    }
    register!(Prepare);
    register!(Launch);
    register!(Inspect);
    register!(Quiesce);
    register!(Remove);
    executor.register::<workload_lifecycle::recording::WaitForSystemApps>(|| {
        workload_lifecycle::recording::WaitForSystemApps
    });
    executor.register::<workload_lifecycle::recording::FinalizeSystemApps>(|| {
        workload_lifecycle::recording::FinalizeSystemApps
    });
}
