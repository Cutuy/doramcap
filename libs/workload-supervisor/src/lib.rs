//! One owned Supervisor daemon and System Monitor recording per run.
mod control;
mod ownership;
pub mod steps;

use orchestrator::{Input, LocalExecutor, Observer, Output};
use prost::Message;
use std::{cell::RefCell, future::Future, path::Path, rc::Rc};
use workload_lifecycle::{
    recorded_run::{self, Phases},
    recording::{FinalizeRecordings, ReadinessInput, Recording, WaitForRecordings},
};

include!(concat!(env!("OUT_DIR"), "/doramcap.supervisor.v1.rs"));
pub type Result<T> = std::result::Result<T, String>;
pub(crate) fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Decode a strict protobuf text-format MonitorRun using its generated descriptor.
pub fn parse_run(text: &str) -> Result<MonitorRun> {
    let pool = prost_reflect::DescriptorPool::decode(
        include_bytes!(concat!(env!("OUT_DIR"), "/supervisor.bin")).as_slice(),
    )
    .map_err(error)?;
    let descriptor = pool
        .get_message_by_name("doramcap.supervisor.v1.MonitorRun")
        .ok_or("MonitorRun descriptor absent")?;
    let message =
        prost_reflect::DynamicMessage::parse_text_format(descriptor, text).map_err(error)?;
    MonitorRun::decode(message.encode_to_vec().as_slice()).map_err(error)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RunReport {
    pub execution_error: Option<String>,
    pub writers_stopped: bool,
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
        let mut phases = SupervisorPhases {
            executor,
            run: run.clone(),
            state,
            prepared: None,
            launched: None,
            stopped: None,
            ready,
        };
        let outcome = recorded_run::run(&mut phases, stop).await;
        let mut diagnostics = outcome.completion_errors;
        if let Some(stopped) = &phases.stopped {
            diagnostics.extend(stopped.value().diagnostics.clone());
        }
        if phases.executor.observer_failures() != 0 {
            diagnostics.push("workflow observer lost events".into());
        }
        let report = RunReport {
            execution_error: outcome.execution_error,
            writers_stopped: outcome.writers_stopped,
            recording_finalized: outcome.recording_finalized,
            cleanup_verified: outcome.cleanup_verified,
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
struct SupervisorPhases {
    executor: LocalExecutor,
    run: MonitorRun,
    state: steps::State,
    prepared: Option<Output<Ownership>>,
    launched: Option<Output<Ownership>>,
    stopped: Option<Output<steps::Quiesced>>,
    ready: Option<tokio::sync::oneshot::Sender<()>>,
}
impl SupervisorPhases {
    fn recordings(&self) -> Vec<Recording> {
        vec![Recording {
            stream: Path::new(&self.run.output_directory).join("cpu.pb"),
            recording: Path::new(&self.run.output_directory).join("cpu.mcap"),
        }]
    }
}
impl Phases for SupervisorPhases {
    async fn launch(&mut self) -> Result<()> {
        let prepared = self
            .executor
            .run::<steps::Prepare>(self.run.clone())
            .await
            .map_err(error)?;
        self.prepared = Some(prepared);
        let launched = self
            .executor
            .run::<steps::Launch>(
                Input::new(self.prepared.as_ref().unwrap().value().clone())
                    .after(self.prepared.as_ref().unwrap()),
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
        self.inspect().await
    }
    fn ready(&mut self) -> Result<()> {
        if let Some(ready) = self.ready.take() {
            ready.send(()).map_err(|_| "readiness receiver dropped")?;
        }
        Ok(())
    }
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
    async fn quiesce(&mut self) -> Result<bool> {
        let mut input = Input::new(self.run.clone());
        if let Some(prepared) = &self.prepared {
            input = input.after(prepared);
        }
        if let Some(launched) = &self.launched {
            input = input.after(launched);
        }
        let stopped = self
            .executor
            .run::<steps::Quiesce>(input)
            .await
            .map_err(error)?;
        let writers_stopped = stopped.value().writers_stopped;
        self.stopped = Some(stopped);
        Ok(writers_stopped)
    }
    async fn finalize(&mut self, writers_stopped: bool) -> Result<()> {
        if !writers_stopped || self.launched.is_none() {
            return Err("recording writer quiescence or launch unverified".into());
        }
        let stopped = self.stopped.as_ref().ok_or("quiescence evidence absent")?;
        if stopped.value().forced {
            return Err("forced shutdown cannot establish recording success".into());
        }
        self.executor
            .run::<FinalizeRecordings>(Input::new(self.recordings()).after(stopped))
            .await
            .map_err(error)?;
        Ok(())
    }
    async fn remove(&mut self) -> Result<bool> {
        let mut input = Input::new(());
        if let Some(stopped) = &self.stopped {
            input = input.after(stopped);
        }
        self.executor
            .run::<steps::Remove>(input)
            .await
            .map(|o| o.into_value())
            .map_err(error)
    }
    async fn finish(&mut self) -> Result<()> {
        // Keep runtime alive through every hook; Drop bounds cancellation cleanup.
        let _ = &self.state;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_pbtxt_and_unsafe_paths_reject_before_launch() {
        // Arrange / Act
        let mut run = parse_run("id:'monitor' output_directory:'/tmp/new' supervisord:'/usr/bin/supervisord' supervisorctl:'/usr/bin/supervisorctl' monitor_binary:'/tmp/monitor' readiness_seconds:10 stop_seconds:2 cgroup_parent:'/sys/fs/cgroup/delegated'").unwrap();
        // Assert
        assert_eq!(run.id, "monitor");
        assert!(parse_run("unexpected_field:1").is_err());
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
