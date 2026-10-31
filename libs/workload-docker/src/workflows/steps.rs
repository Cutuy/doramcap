//! Shared registration of Docker workflow steps on an orchestrator executor.
use crate::compose::{
    Diagnostics, DockerAdapter, InspectCompose, QuiesceCompose, RemoveCompose, ResolveCompose,
    StartCompose,
    filesystem::{CaptureFilesystemBaseline, CollectFilesystemChanges},
};
use orchestrator::{LocalExecutor, Observer};
use std::rc::{Rc, Weak};

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

/// Register communication, graph and workload operations on the experiment executor.
pub fn register_experiment(
    executor: &mut LocalExecutor,
    owner: super::experiment::SharedExperiment,
) {
    use super::experiment::{RegisterDoraGraph, StartDoraRuntime, StopDoraRuntime};
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

/// Build the recorded-run executor with shared mechanics and session steps.
/// Weak child-call references keep factories from retaining their own executor.
pub(super) fn executor(
    adapter: Rc<DockerAdapter>,
    diagnostics: Diagnostics,
    observer: impl Observer + 'static,
    label: &str,
) -> Rc<LocalExecutor> {
    attach(
        LocalExecutor::new(observer)
            .with_label(label)
            .capture_payloads(true),
        adapter,
        diagnostics,
    )
}

/// Bind prepared experiment operations while preserving preparation event lineage.
pub(super) fn bind_experiment(
    executor: Rc<LocalExecutor>,
    adapter: Rc<DockerAdapter>,
    diagnostics: Diagnostics,
    owner: super::experiment::SharedExperiment,
) -> crate::compose::Result<Rc<LocalExecutor>> {
    let mut executor = Rc::try_unwrap(executor)
        .map_err(|_| crate::compose::error::invalid("executor still running during binding"))?;
    register_experiment(&mut executor, owner);
    Ok(attach(executor, adapter, diagnostics))
}

pub(super) fn attach(
    mut executor: LocalExecutor,
    adapter: Rc<DockerAdapter>,
    diagnostics: Diagnostics,
) -> Rc<LocalExecutor> {
    use super::recorded_compose::{finalization::*, preparation::PrepareRun};
    use crate::compose::overlay::{FinalizeOverlays, PrepareOverlays};
    Rc::new_cyclic(move |weak: &Weak<LocalExecutor>| {
        register(&mut executor, adapter.clone(), diagnostics.clone());
        let dependency = adapter.clone();
        executor.register::<PrepareOverlays>(move || PrepareOverlays {
            adapter: dependency.clone(),
        });
        executor.register::<FinalizeOverlays>(|| FinalizeOverlays);
        use workload_lifecycle::recording::{FinalizeRecordings, WaitForRecordings};
        executor.register::<WaitForRecordings>(|| WaitForRecordings);
        executor.register::<FinalizeRecordings>(|| FinalizeRecordings);
        let dependency = adapter.clone();
        let child = weak.clone();
        executor.register::<PrepareRun>(move || PrepareRun {
            adapter: dependency.clone(),
            executor: child.clone(),
        });
        let child = weak.clone();
        let sink = diagnostics.clone();
        executor.register::<FinalizeRun>(move || FinalizeRun {
            executor: child.clone(),
            diagnostics: sink.clone(),
        });
        executor.register::<WriteRunReport>(move || WriteRunReport {
            diagnostics: diagnostics.clone(),
        });
        executor
    })
}
