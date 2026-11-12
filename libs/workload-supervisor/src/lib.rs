//! One owned Supervisor daemon and System Monitor recording per run.
mod control;
mod ownership;
pub mod steps;

use orchestrator::{Input, LocalExecutor, Observer, Output};
use std::{
    cell::RefCell,
    future::Future,
    io,
    os::fd::{AsRawFd, OwnedFd},
    path::Path,
    rc::Rc,
};
use tokio::io::{Interest, unix::AsyncFd};
use workload_lifecycle::{
    launch::{CleanupRecord, RemovalPermit, StopRecord},
    recording::{ReadinessInput, Recording, WaitForRecordings},
    run::{self, Running},
};

include!(concat!(env!("OUT_DIR"), "/doramcap.supervisor.v1.rs"));
pub type Result<T> = std::result::Result<T, String>;
pub(crate) fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RunReport {
    pub execution_error: Option<String>,
    pub writers_stopped: bool,
    /// Files passed validation after a completed launch and unforced shutdown.
    pub recording_finalized: bool,
    pub cleanup_verified: bool,
    pub diagnostics: Vec<String>,
}
impl RunReport {
    pub fn successful(&self) -> bool {
        self.execution_error.is_none()
            && self.writers_stopped
            && self.recording_finalized
            && self.cleanup_verified
            && self.diagnostics.is_empty()
    }
}

/// Serial admission on one manager. A dropped run leaves the manager blocked;
/// cooperative stop awaits teardown. Existing output/cgroup paths reject reuse.
#[derive(Default)]
pub struct SupervisorRunner {
    blocked: bool,
}
impl SupervisorRunner {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn blocked(&self) -> bool {
        self.blocked
    }
    /// Run prepared settings until cooperative stop, then await cleanup.
    /// Returns execution failures in `Ok(RunReport)`; inspect its independent outcomes.
    /// See [`Self::run_with_ready`] for errors, observer ownership and cancellation.
    pub async fn run(
        &mut self,
        run: MonitorRun,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
    ) -> Result<RunReport> {
        self.run_with_ready(run, stop, observer, None).await
    }
    /// Run and notify after recording readiness and liveness succeed.
    ///
    /// # Results and errors
    /// Preflight/admission errors return `Err`; execution and teardown failures enter
    /// `Ok(RunReport)`. Evidence-writing errors return `Err` and keep admission blocked.
    ///
    /// # Ownership and cancellation
    /// Owns `observer`, which receives synchronous events with serialized inputs.
    /// A closed readiness receiver triggers teardown. Signal `stop` and await cleanup;
    /// dropping the future performs best-effort termination and leaves admission blocked.
    pub async fn run_with_ready(
        &mut self,
        run: MonitorRun,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
        ready: Option<tokio::sync::oneshot::Sender<()>>,
    ) -> Result<RunReport> {
        if self.blocked {
            return Err("manager blocked by unfinished ownership or cleanup".into());
        }
        steps::validate(&run)?;
        self.blocked = true;
        let state = Rc::new(RefCell::new(None));
        let mut executor = LocalExecutor::new(observer)
            .with_label(&run.id)
            .capture_payloads(true);
        steps::register(&mut executor, state.clone());
        let cgroup = Rc::new(workload_lifecycle::cgroup::Group::new(Rc::new(
            steps::CgroupBackend {
                state: state.clone(),
            },
        )));
        workload_lifecycle::cgroup::register(&mut executor, cgroup.clone());
        let executor = Rc::new(executor);
        let running = SupervisorRunning {
            executor: executor.clone(),
            cgroup,
            run: run.clone(),
            state,
            prepared: None,
            launched: None,
            stopped: None,
            ready,
            exits: None,
            diagnostics: Vec::new(),
        };
        let recordings = running.recordings();
        let (outcome, removed) = run::run(running, &executor, recordings, stop).await;
        let mut diagnostics = outcome.diagnostics;
        diagnostics.extend(removed.diagnostics);
        if let Some(stopped) = &removed.stopped {
            diagnostics.extend(stopped.diagnostics.clone());
        }
        let recording_allowed = if !outcome.stopped.writers_stopped || !removed.launched {
            diagnostics.push("recording writer quiescence or launch unverified".into());
            false
        } else if removed
            .stopped
            .as_ref()
            .is_some_and(|stopped| stopped.forced)
        {
            diagnostics.push("forced shutdown cannot establish recording success".into());
            false
        } else {
            true
        };
        if executor.observer_failures() != 0 {
            diagnostics.push("workflow observer lost events".into());
        }
        let report = RunReport {
            execution_error: outcome.execution_error,
            writers_stopped: outcome.stopped.writers_stopped,
            recording_finalized: outcome.permit.recording_finalized && recording_allowed,
            cleanup_verified: outcome.cleanup.verified,
            diagnostics,
        };
        // No report can erase the durable ownership record; evidence remains beside outputs.
        if Path::new(&run.output_directory).exists() {
            ownership::private_file(
                &Path::new(&run.output_directory).join("report.json"),
                &serde_json::to_vec_pretty(&report).map_err(error)?,
            )?;
        }
        self.blocked = !report.cleanup_verified;
        Ok(report)
    }
}
struct SupervisorRunning {
    executor: Rc<LocalExecutor>,
    cgroup: Rc<workload_lifecycle::cgroup::Group>,
    run: MonitorRun,
    state: steps::State,
    prepared: Option<Output<Ownership>>,
    launched: Option<Output<Ownership>>,
    stopped: Option<Output<steps::Quiesced>>,
    ready: Option<tokio::sync::oneshot::Sender<()>>,
    exits: Option<(AsyncFd<OwnedFd>, AsyncFd<OwnedFd>)>,
    diagnostics: Vec<String>,
}

struct Removed {
    launched: bool,
    stopped: Option<steps::Quiesced>,
    diagnostics: Vec<String>,
}
impl SupervisorRunning {
    fn recordings(&self) -> Vec<Recording> {
        vec![Recording {
            stream: Path::new(&self.run.output_directory).join("cpu.pb"),
            recording: Path::new(&self.run.output_directory).join("cpu.mcap"),
        }]
    }
}
impl Running for SupervisorRunning {
    type Removed = Removed;

    async fn start(&mut self) -> Result<()> {
        let prepared = self
            .executor
            .run::<steps::Prepare>(self.run.clone())
            .await
            .map_err(error)?;
        self.prepared = Some(prepared);
        let cgroup = self
            .executor
            .run::<workload_lifecycle::cgroup::SystemAppsCgroups>(
                Input::new(workload_lifecycle::launch::SystemAppsCgroupsInput {
                    action: workload_lifecycle::cgroup::Action::Create.into(),
                    artifacts: self.run.output_directory.clone(),
                    readiness_seconds: self.run.readiness_seconds,
                    pids: Vec::new(),
                })
                .after(self.prepared.as_ref().unwrap()),
            )
            .await
            .map_err(error)?;
        let launched = self
            .executor
            .run::<steps::Launch>(
                Input::new(self.prepared.as_ref().unwrap().value().clone())
                    .after(self.prepared.as_ref().unwrap())
                    .after(&cgroup),
            )
            .await
            .map_err(error)?;
        self.launched = Some(launched);
        let input = Input::new(ReadinessInput {
            recordings: self.recordings(),
            timeout_seconds: self.run.readiness_seconds,
        })
        .after(self.launched.as_ref().unwrap());
        self.executor
            .run::<WaitForRecordings>(input)
            .await
            .map_err(error)?;
        let (daemon_pid, monitor_pid) = {
            let state = self.state.borrow();
            let ownership = &state.as_ref().unwrap().ledger;
            (ownership.daemon_pid, ownership.monitor_pid)
        };
        let observe = |pid: u32| -> Result<AsyncFd<OwnedFd>> {
            let pid = rustix::process::Pid::from_raw(pid.try_into().map_err(error)?)
                .ok_or("process ID absent")?;
            let fd = rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::empty())
                .map_err(error)?;
            // SAFETY: OwnedFd keeps this descriptor open and unchanged until it is dropped.
            unsafe { AsyncFd::register_with_interest(fd, Interest::READABLE) }.map_err(error)
        };
        self.cgroup.verify_pid(daemon_pid)?;
        self.cgroup.verify_pid(monitor_pid)?;
        self.exits = Some((observe(daemon_pid)?, observe(monitor_pid)?));
        self.inspect().await
    }
    fn ready(&mut self) -> Result<()> {
        if let Some(ready) = self.ready.take() {
            ready.send(()).map_err(|_| "readiness receiver dropped")?;
        }
        Ok(())
    }
    async fn ended(&mut self) -> String {
        if let Err(error) = self.inspect().await {
            return error;
        }
        self.wait_for_change().await
    }
    async fn stop(&mut self) -> StopRecord {
        let mut input = Input::new(self.run.clone());
        if let Some(prepared) = &self.prepared {
            input = input.after(prepared);
        }
        if let Some(launched) = &self.launched {
            input = input.after(launched);
        }
        match self.executor.run::<steps::Quiesce>(input).await {
            Ok(stopped) => {
                let record = StopRecord {
                    writers_stopped: stopped.value().writers_stopped,
                    evidence_durable: true,
                };
                self.stopped = Some(stopped);
                record
            }
            Err(error) => {
                self.diagnostics.push(error.to_string());
                StopRecord::default()
            }
        }
    }

    async fn remove(mut self, permit: &RemovalPermit) -> (CleanupRecord, Removed) {
        let verified = if permit.remove {
            let mut input = Input::new(workload_lifecycle::launch::SystemAppsCgroupsInput {
                action: workload_lifecycle::cgroup::Action::Remove.into(),
                ..Default::default()
            });
            if let Some(stopped) = &self.stopped {
                input = input.after(stopped);
            }
            match self
                .executor
                .run::<workload_lifecycle::cgroup::SystemAppsCgroups>(input)
                .await
            {
                Ok(output) => output.value().is_none(),
                Err(error) => {
                    self.diagnostics.push(error.to_string());
                    false
                }
            }
        } else {
            self.diagnostics
                .push("preserved: writer quiescence or durable evidence unverified".into());
            false
        };
        (
            CleanupRecord { verified },
            Removed {
                launched: self.launched.is_some(),
                stopped: self.stopped.map(Output::into_value),
                diagnostics: self.diagnostics,
            },
        )
    }
}

impl SupervisorRunning {
    async fn inspect(&mut self) -> Result<()> {
        let mut input = Input::new(self.run.clone());
        if let Some(launched) = &self.launched {
            input = input.after(launched);
        }
        self.executor
            .run::<steps::Inspect>(input)
            .await
            .map_err(error)?;
        Ok(())
    }
    async fn wait_for_change(&self) -> String {
        let Some((daemon, monitor)) = &self.exits else {
            return "process-exit handles absent".into();
        };
        tokio::select! {
            exit = wait_for_exit(daemon) => {
                exit.map_or_else(error, |_| "Supervisor exited early".into())
            }
            exit = wait_for_exit(monitor) => {
                exit.map_or_else(error, |_| "monitor exited early".into())
            }
        }
    }
}

/// Confirm pidfd readiness; spurious notifications return to the reactor.
async fn wait_for_exit(process: &AsyncFd<OwnedFd>) -> io::Result<()> {
    const CHECK_CURRENT_READINESS: libc::c_int = 0;
    const PIDFD_COUNT: libc::nfds_t = 1;
    process
        .async_io(Interest::READABLE, |fd| {
            let mut descriptor = libc::pollfd {
                fd: fd.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: descriptor is initialized and valid for one entry throughout poll.
            let ready =
                unsafe { libc::poll(&mut descriptor, PIDFD_COUNT, CHECK_CURRENT_READINESS) };
            if ready < 0 {
                return Err(io::Error::last_os_error());
            }
            if descriptor.revents & (libc::POLLIN | libc::POLLHUP) != 0 {
                Ok(())
            } else {
                Err(io::ErrorKind::WouldBlock.into())
            }
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn process_exit_ends_observation_before_supervisor_status_catches_up() {
        use std::{process::Stdio, time::Duration};
        const EXIT_DEADLINE: Duration = Duration::from_secs(5);

        for (exited_process, expected_error) in
            [(0, "Supervisor exited early"), (1, "monitor exited early")]
        {
            // Arrange: both processes stay alive until explicitly killed.
            let mut children = [0, 1].map(|_| {
                tokio::process::Command::new("/bin/cat")
                    .stdin(Stdio::piped())
                    .kill_on_drop(true)
                    .spawn()
                    .unwrap()
            });
            let exits = children.each_ref().map(|child| {
                let pid = rustix::process::Pid::from_raw(child.id().unwrap() as i32).unwrap();
                let fd =
                    rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::empty()).unwrap();
                // SAFETY: OwnedFd retains the unchanged pidfd until the wait finishes.
                unsafe { AsyncFd::register_with_interest(fd, Interest::READABLE) }.unwrap()
            });
            let [daemon, monitor] = exits;
            let running = SupervisorRunning {
                executor: Rc::new(LocalExecutor::new(|_: &orchestrator::Event| {})),
                run: MonitorRun::default(),
                cgroup: Rc::new(workload_lifecycle::cgroup::Group::new(Rc::new(
                    steps::CgroupBackend {
                        state: Rc::new(RefCell::new(None)),
                    },
                ))),
                state: Rc::new(RefCell::new(None)),
                prepared: None,
                launched: None,
                stopped: None,
                ready: None,
                exits: Some((daemon, monitor)),
                diagnostics: Vec::new(),
            };
            let wait = running.wait_for_change();
            tokio::pin!(wait);

            // Act / Assert: observation stays pending while both processes live.
            tokio::select! {
                biased;
                result = &mut wait => panic!("live processes ended observation: {result:?}"),
                _ = std::future::ready(()) => {}
            }

            // Act: exit is observable before any Supervisor status inspection.
            children[exited_process].start_kill().unwrap();
            let result = tokio::time::timeout(EXIT_DEADLINE, &mut wait)
                .await
                .expect("process exit did not wake observation");

            // Assert: exit ends execution immediately, allowing the driver to clean up.
            assert_eq!(result, expected_error);
            for child in &mut children {
                child.kill().await.unwrap();
            }
        }
    }

    #[test]
    fn unsafe_paths_reject_before_launch() {
        // Arrange
        let mut run = MonitorRun {
            id: "monitor".into(),
            output_directory: "/tmp/new".into(),
            supervisord: "/usr/bin/supervisord".into(),
            supervisorctl: "/usr/bin/supervisorctl".into(),
            monitor_binary: "/tmp/monitor".into(),
            readiness_seconds: 10,
            stop_seconds: 2,
            cgroup_parent: "/sys/fs/cgroup/delegated".into(),
        };
        for path in [
            "/tmp/percent%path",
            "/tmp/quote\"path",
            "/tmp/comment;path",
            "/tmp/new\nline",
            "relative/path",
        ] {
            // Arrange
            run.output_directory = path.into();
            // Act / Assert
            assert!(
                steps::validate(&run)
                    .unwrap_err()
                    .contains("absolute paths")
            );
        }
        // Arrange
        run.output_directory = "/tmp/fresh-supervisor-preflight".into();
        run.monitor_binary = "/nonexistent/system-monitor".into();
        // Act / Assert
        assert!(steps::validate(&run).is_err());
    }
}
