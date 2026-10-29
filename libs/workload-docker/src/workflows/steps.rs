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

/// Build the recorded-run executor with shared mechanics and session steps.
/// Weak child-call references keep factories from retaining their own executor.
pub(super) fn executor(
    adapter: Rc<DockerAdapter>,
    diagnostics: Diagnostics,
    observer: impl Observer + 'static,
    label: &str,
) -> Rc<LocalExecutor> {
    use super::{
        recorded_compose::{finalization::*, preparation::PrepareRun, projects::*},
        system_apps::*,
    };
    use crate::compose::overlay::{FinalizeOverlays, PrepareOverlays};
    Rc::new_cyclic(|weak: &Weak<LocalExecutor>| {
        let mut executor = LocalExecutor::new(observer)
            .with_label(label)
            .capture_payloads(true);
        register(&mut executor, adapter.clone(), diagnostics.clone());
        super::dora::register(&mut executor, adapter.clone());
        let dependency = adapter.clone();
        executor.register::<PrepareOverlays>(move || PrepareOverlays {
            adapter: dependency.clone(),
        });
        executor.register::<FinalizeOverlays>(|| FinalizeOverlays);
        executor.register::<PrepareSystemApps>(|| PrepareSystemApps);
        executor.register::<WaitForSystemApps>(|| WaitForSystemApps);
        executor.register::<FinalizeSystemApps>(|| FinalizeSystemApps);
        let dependency = adapter.clone();
        let child = weak.clone();
        executor.register::<PrepareRun>(move || PrepareRun {
            adapter: dependency.clone(),
            executor: child.clone(),
        });
        macro_rules! nested {
            ($kind:ident) => {{
                let child = weak.clone();
                executor.register::<$kind>(move || $kind {
                    executor: child.clone(),
                });
            }};
        }
        nested!(LaunchProjects);
        nested!(LaunchDoraFlows);
        nested!(ObserveProjects);
        nested!(RemoveProjects);
        let child = weak.clone();
        let sink = diagnostics.clone();
        executor.register::<QuiesceProjects>(move || QuiesceProjects {
            executor: child.clone(),
            diagnostics: sink.clone(),
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
