//! Shared registration of Docker workflow steps on an orchestrator executor.
use crate::compose::{
    Diagnostics, DockerAdapter, InspectCompose, QuiesceCompose, RemoveCompose, ResolveCompose,
    StartCompose,
    filesystem::{CaptureFilesystemBaseline, CollectFilesystemChanges},
};
use orchestrator::LocalExecutor;
use std::rc::Rc;

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
    let dependency = adapter.clone();
    executor.register::<CaptureFilesystemBaseline>(move || CaptureFilesystemBaseline {
        adapter: dependency.clone(),
    });
    executor.register::<CollectFilesystemChanges>(move || CollectFilesystemChanges {
        adapter: adapter.clone(),
    });
}
