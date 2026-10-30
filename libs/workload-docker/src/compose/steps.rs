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
    ($name:ident, $input:ty, $output:ty, $method:ident, $description:literal) => {
        #[doc = $description]
        pub struct $name {
            pub adapter: Rc<DockerAdapter>,
            pub diagnostics: Diagnostics,
        }
        impl Step for $name {
            type Input = $input;
            type Output = $output;
            async fn run(&mut self, input: Self::Input) -> Result<Self::Output> {
                self.adapter.$method(input).await.map_err(|error| {
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
    resolve,
    "Resolve prepared project settings and freeze its launch manifest and image IDs."
);
step!(
    StartCompose,
    StartInput,
    ComposeProjectState,
    start,
    "Start one project; caller must clean the attempted plan on every failure path."
);
step!(
    StopCompose,
    StopInput,
    Cleanup,
    stop,
    "Stop and reconcile one project; inspect cleanup diagnostics and verified status."
);

/// Return actual container identities, PIDs, health and exit state for one project.
pub struct InspectCompose {
    pub adapter: Rc<DockerAdapter>,
    pub diagnostics: Diagnostics,
}
impl Step for InspectCompose {
    fn descriptor() -> Option<orchestrator::StepDescriptor> {
        Some(orchestrator::StepDescriptor::new(
            "compose.inspect",
            "Inspect project",
            orchestrator::ProgressRetention::Recent,
            orchestrator::ProgressKind::Observation,
        ))
    }
    type Input = ResolvedComposeProject;
    type Output = ComposeProjectState;
    async fn run(&mut self, plan: ResolvedComposeProject) -> Result<ComposeProjectState> {
        self.adapter.inspect(&plan).await.map_err(|error| {
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
    quiesce,
    "Stop writers while preserving containers and volumes for filesystem evidence."
);
step!(
    RemoveCompose,
    StopInput,
    Cleanup,
    remove,
    "Remove stopped project resources after evidence collection."
);
