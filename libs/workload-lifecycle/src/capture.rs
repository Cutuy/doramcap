//! Concrete filesystem steps shared by workload backends.
use host_filesystem::{
    changes::{self, CollectInput, FilesystemChanges, FilesystemSources, SourcesInput},
    overlay::{self, OverlayResults, OverlaySet},
};
use orchestrator::Step;

/// Prepare source identities and coverage without walking folders or mounting views.
pub struct PrepareFilesystemSources;
impl Step for PrepareFilesystemSources {
    const ID: Option<&'static str> =
        Some("workload_docker::compose::filesystem::CaptureFilesystemBaseline");
    type Input = SourcesInput;
    type Output = FilesystemSources;
    async fn run(&mut self, mut input: Self::Input) -> orchestrator::Result<Self::Output> {
        input.excluded.push(crate::journal::DIRECTORY.into());
        changes::prepare_sources(input)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
/// Save metadata after the backend has stopped and verified every writer.
/// Collection errors retain resources needed for recovery.
pub struct CollectFilesystemChanges;
impl Step for CollectFilesystemChanges {
    const ID: Option<&'static str> =
        Some("workload_docker::compose::filesystem::CollectFilesystemChanges");
    type Input = CollectInput;
    type Output = FilesystemChanges;
    async fn run(&mut self, input: Self::Input) -> orchestrator::Result<Self::Output> {
        changes::collect(input)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
/// Strictly unmount stopped views, then apply or discard upper changes.
/// Interrupted write-back retains upper data; reset never replays writes.
pub struct FinalizeOverlays;
impl Step for FinalizeOverlays {
    const ID: Option<&'static str> = Some("workload_docker::compose::overlay::FinalizeOverlays");
    type Input = OverlaySet;
    type Output = OverlayResults;
    async fn run(&mut self, input: Self::Input) -> orchestrator::Result<Self::Output> {
        overlay::finalize(input)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}

/// Collection and finalization outcomes; either failure keeps owned resources recoverable.
pub struct CaptureFinalization {
    pub changes: Option<FilesystemChanges>,
    pub results: Option<OverlayResults>,
    pub failures: Vec<(&'static str, String)>,
}
/// Collect stopped sources before destructive finalization, retaining their step dependency.
/// Backend step outputs retain their dependency on collection; callers own checkpoints and removal.
pub async fn finalize(
    executor: &orchestrator::LocalExecutor,
    input: impl Into<orchestrator::Input<CollectInput>>,
    overlays: Option<OverlaySet>,
) -> CaptureFinalization {
    let changes = match executor.run::<CollectFilesystemChanges>(input).await {
        Ok(changes) => changes,
        Err(error) => {
            return CaptureFinalization {
                changes: None,
                results: None,
                failures: vec![("filesystem changes", error.to_string())],
            };
        }
    };
    let mut failures = Vec::new();
    let results = if let Some(overlays) = overlays {
        match executor
            .run::<FinalizeOverlays>(orchestrator::Input::new(overlays).after(&changes))
            .await
        {
            Ok(results) => Some(results.into_value()),
            Err(error) => {
                failures.push(("finalize overlays", error.to_string()));
                None
            }
        }
    } else {
        None
    };
    CaptureFinalization {
        changes: Some(changes.into_value()),
        results,
        failures,
    }
}
