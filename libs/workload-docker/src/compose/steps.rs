use super::{
    Cleanup, ComposeProjectState, Diagnostic, DockerAdapter, ResolveInput, ResolvedComposeProject,
    Shutdown, StartInput, StopInput,
};
use orchestrator::{Result, Step};
use std::{cell::RefCell, rc::Rc};

// Diagnostics preserve typed adapter details alongside the orchestrator's textual
// terminal error. This is an explicitly injected evidence sink, never a context bag.
pub type Diagnostics = Rc<RefCell<Vec<Diagnostic>>>;
macro_rules! step {
    ($name:ident, $input:ty, $output:ty, $adapter:ident, $request:ident, $operation:expr, $description:literal) => {
        #[doc = $description]
        pub struct $name {
            pub adapter: Rc<DockerAdapter>,
            pub diagnostics: Diagnostics,
        }
        impl Step for $name {
            type Input = $input;
            type Output = $output;
            async fn run(&mut self, $request: Self::Input) -> Result<Self::Output> {
                let $adapter = &self.adapter;
                let result: super::Result<_> = async { $operation }.await;
                result.map_err(|error| {
                    self.diagnostics
                        .borrow_mut()
                        .push(error.diagnostic(stringify!($name)));
                    orchestrator::Error::message(error.to_string())
                })
            }
        }
    };
}
step!(
    ResolveCompose,
    ResolveInput,
    ResolvedComposeProject,
    adapter,
    input,
    adapter.configuration().resolve(input).await,
    "Resolve prepared project settings and freeze its launch manifest and image IDs."
);
step!(
    StartCompose,
    StartInput,
    ComposeProjectState,
    adapter,
    input,
    adapter
        .project(&input.plan)
        .start(&input.services, input.readiness_seconds)
        .await,
    "Start one project; caller must clean the attempted plan on every failure path."
);
step!(
    StopCompose,
    StopInput,
    Cleanup,
    adapter,
    input,
    {
        if !input.services.is_empty() {
            return Err(super::error::invalid(
                "selected services use quiesce; removal owns the whole experiment",
            ));
        }
        adapter
            .project(&input.plan)
            .stop(input.stop_seconds, input.allow_volume_deletion)
            .await
    },
    "Stop and reconcile one project; inspect cleanup diagnostics and verified status."
);

/// Return actual container identities, PIDs, health and exit state for one project.
pub struct InspectCompose {
    pub adapter: Rc<DockerAdapter>,
    pub diagnostics: Diagnostics,
}
impl Step for InspectCompose {
    const ID: Option<&'static str> = Some("compose.inspect");
    const DISPLAY_NAME: Option<&'static str> = Some("Inspect project");
    type Input = ResolvedComposeProject;
    type Output = ComposeProjectState;
    async fn run(&mut self, plan: ResolvedComposeProject) -> Result<ComposeProjectState> {
        self.adapter
            .project(&plan)
            .inspect()
            .await
            .map_err(|error| {
                self.diagnostics
                    .borrow_mut()
                    .push(error.diagnostic("InspectCompose"));
                orchestrator::Error::message(error.to_string())
            })
    }
}

step!(
    QuiesceCompose,
    StopInput,
    Shutdown,
    adapter,
    input,
    adapter
        .project(&input.plan)
        .quiesce(&input.services, input.stop_seconds)
        .await,
    "Stop writers while preserving containers and volumes for filesystem evidence."
);
step!(
    RemoveCompose,
    StopInput,
    Cleanup,
    adapter,
    input,
    {
        if !input.services.is_empty() {
            return Err(super::error::invalid(
                "removal requires the whole experiment",
            ));
        }
        adapter
            .project(&input.plan)
            .remove(input.allow_volume_deletion)
            .await
    },
    "Remove stopped project resources after evidence collection."
);
