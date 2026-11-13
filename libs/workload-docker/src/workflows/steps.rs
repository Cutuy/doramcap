//! Registration of the Compose workflow and its finalization steps.
use compose_apps::{
    compose::{Diagnostics, DockerAdapter},
    steps::{register, register_experiment},
};
use orchestrator::{LocalExecutor, Observer};
use std::rc::{Rc, Weak};

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
    owner: compose_apps::experiment::SharedExperiment,
) -> compose_apps::compose::Result<Rc<LocalExecutor>> {
    let mut executor = Rc::try_unwrap(executor).map_err(|_| {
        compose_apps::compose::error::invalid("executor still running during binding")
    })?;
    register_experiment(&mut executor, owner);
    Ok(attach(executor, adapter, diagnostics))
}

pub(super) fn attach(
    mut executor: LocalExecutor,
    adapter: Rc<DockerAdapter>,
    diagnostics: Diagnostics,
) -> Rc<LocalExecutor> {
    use super::recorded_compose::finalization::*;
    use compose_apps::compose::PrepareOverlays;
    use compose_apps::preparation::PrepareRun;
    use workload_lifecycle::capture::FinalizeOverlays;
    Rc::new_cyclic(move |weak: &Weak<LocalExecutor>| {
        register(&mut executor, adapter.clone(), diagnostics.clone());
        let dependency = adapter.clone();
        executor.register::<PrepareOverlays>(move || PrepareOverlays {
            adapter: dependency.clone(),
        });
        executor.register::<FinalizeOverlays>(|| FinalizeOverlays);
        use workload_lifecycle::recording::FinalizeRecordings;
        executor.register::<FinalizeRecordings>(|| FinalizeRecordings);
        executor.register::<workload_lifecycle::journal::ExportJournal>(|| {
            workload_lifecycle::journal::ExportJournal
        });
        let dependency = adapter.clone();
        let child = weak.clone();
        executor.register::<PrepareRun>(move || PrepareRun {
            adapter: dependency.clone(),
            executor: child.clone(),
        });
        let child = weak.clone();
        let sink = diagnostics.clone();
        let observation_adapter = adapter.clone();
        executor.register::<ObserveDockerFilesystem>(move || ObserveDockerFilesystem {
            adapter: observation_adapter.clone(),
        });
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
