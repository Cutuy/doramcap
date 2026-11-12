//! Shared registration of Docker workflow steps on an orchestrator executor.
use crate::compose::{
    Diagnostics, DockerAdapter, InspectCompose, QuiesceCompose, RemoveCompose, ResolveCompose,
    StartCompose,
};
use orchestrator::LocalExecutor;
use std::rc::Rc;
use workload_lifecycle::capture::{CaptureFilesystemBaseline, CollectFilesystemChanges};

/// Register lifecycle and filesystem steps with shared adapter and diagnostics.
/// The caller retains its executor's observer, label and payload policy.
pub fn register(
    executor: &mut LocalExecutor,
    adapter: Rc<DockerAdapter>,
    diagnostics: Diagnostics,
) {
    macro_rules! step {
        ($kind:ident) => {{
            let adapter = adapter.clone();
            let diagnostics = diagnostics.clone();
            executor.register::<$kind>(move || $kind {
                adapter: adapter.clone(),
                diagnostics: diagnostics.clone(),
            });
        }};
    }
    step!(ResolveCompose);
    step!(StartCompose);
    step!(InspectCompose);
    step!(QuiesceCompose);
    step!(RemoveCompose);
    executor.register::<CaptureFilesystemBaseline>(|| CaptureFilesystemBaseline);
    executor.register::<CollectFilesystemChanges>(|| CollectFilesystemChanges);
}

/// Register communication, graph and workload operations on the experiment executor.
pub fn register_experiment(
    executor: &mut LocalExecutor,
    owner: crate::experiment::SharedExperiment,
) {
    use crate::experiment::{RegisterDoraGraph, StartDoraRuntime, StopDoraRuntime};
    macro_rules! step {
        ($kind:ident, $dependency:ident) => {{
            let owner = $dependency.clone();
            executor.register::<$kind>(move || $kind {
                owner: owner.clone(),
            });
        }};
    }
    step!(StartDoraRuntime, owner);
    step!(StopDoraRuntime, owner);
    step!(RegisterDoraGraph, owner);
}
