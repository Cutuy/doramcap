//! Observable application-run reset over the backend's shared Build/Run cleanup policy.
use docker_engine::Engine;
use orchestrator::{Error, Step};
use workload_lifecycle::launch::RunPlan;

/// Stop previous owned writers and discard their views while the caller retains admission.
/// Shared reset never captures changes or repeats write-back; uncertain cleanup blocks launch.
pub(super) struct ResetProject {
    pub engine: Engine,
}
impl Step for ResetProject {
    const ID: Option<&'static str> = Some("holoscan.reset-project");
    const DISPLAY_NAME: Option<&'static str> = Some("Reset Holoscan project");
    type Input = RunPlan;
    type Output = ();
    async fn run(&mut self, run: RunPlan) -> orchestrator::Result<()> {
        crate::reset::reset_project(&self.engine, &run)
            .await
            .map_err(Error::message)
    }
}
