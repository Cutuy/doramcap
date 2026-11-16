//! Partial native Review: verified inputs and recorded artifacts, with unavailable launch details explicit.
use crate::{
    BuildState, Catalog, CatalogOutcome, ConfigurationSnapshot, DiscoverApplications,
    DiscoverRequest, NativeInputFile, Preparation, PrepareRequest, RecordedImageObservation,
    RecordedImageState, Selection,
    build_record::assess_build_record,
    discovery::{cancelled, normalize_language, query},
    project::read_project_inputs,
    query_diagnostic::Termination,
    routing::verify_native_daemon,
};
use docker_engine::Engine;
use orchestrator::{Error, Result, Step};
use prost::Message;
use std::{collections::BTreeMap, path::Path};
use tokio::sync::watch;
use workload_lifecycle::launch::{HostShmAccess, RunPlan};

/// Observe native selection queries, raw files, Docker coherence and the recorded Build artifact.
/// Human dry-run streams remain partial diagnostics, never reconstructed launch arguments.
/// No build, pull, container launch, overlay or write is performed.
///
/// # Inputs and ownership
/// The tuple pairs the common RunPlan with generated backend inputs. The supplied Build record
/// must belong to the same canonical repository root as admission. Capture/store ownership stays
/// with the caller; absent source facts are supported and do not independently block Review.
///
/// # Scope and outcomes
/// PASSTHROUGH is required: unknown native Docker options cannot prove BLOCKED. Recording,
/// Dora and system-app requests are rejected until their integration exists. Rejections retain
/// observations gathered so far. Unknown native defaults, next-launch image, mounts, limits,
/// logging and execution location are explicit; the snapshot covers only the verified inputs.
/// Its digest excludes human output and generated run/artifact identity.
///
/// # Cancellation and errors
/// With a shutdown receiver, true/disconnect cancels Engine reads and kills/reaps an active direct
/// CLI child. The owner must await completion before closing its runtime. Dropping only kills
/// that child. Cleanup failures propagate as errors; query failures retain stdout/stderr.
pub struct PrepareLaunch {
    engine: Engine,
    shutdown: Option<watch::Receiver<bool>>,
}
impl PrepareLaunch {
    /// Use this selected Engine for read-only daemon/artifact observations.
    pub fn new(engine: Engine) -> Self {
        Self {
            engine,
            shutdown: None,
        }
    }
    /// Use owner shutdown for cancellation; the owner must join the invocation before runtime close.
    pub fn with_shutdown(engine: Engine, shutdown: watch::Receiver<bool>) -> Self {
        Self {
            engine,
            shutdown: Some(shutdown),
        }
    }
}
impl Step for PrepareLaunch {
    const ID: Option<&'static str> = Some("holoscan.prepare-launch");
    const DISPLAY_NAME: Option<&'static str> = Some("Review Holoscan launch facts");
    type Input = (RunPlan, PrepareRequest);
    type Output = Preparation;

    async fn run(&mut self, (run, request): Self::Input) -> Result<Preparation> {
        let plan = request
            .plan
            .ok_or_else(|| Error::Configuration("Holoscan plan is required".into()))?;
        let selection = plan
            .selection
            .as_ref()
            .ok_or_else(|| Error::Configuration("Holoscan selection is required".into()))?;
        let mut result = Preparation {
            build: Some(assess_build_record(request.build_record.as_ref(), selection, request.current_source.as_ref())),
            unknown: vec![
                "Native next-launch image and execution location are unknown".into(),
                "Effective native project context and application of declared defaults are unknown".into(),
                "Native mounts and writable coverage are unknown; an empty list does not establish absence".into(),
                "Native log driver, resource limits and conflicting Docker controls are unknown".into(),
            ],
            warnings: vec!["Review contains verified input and Build-artifact facts only; native dry-run text is partial diagnostic evidence".into()],
            ..Default::default()
        };
        if selection.language.is_empty() {
            result
                .unknown
                .push("Native default language is unknown".into());
        }
        if selection.mode.is_empty() {
            result.unknown.push("Native default mode is unknown".into());
        }
        if let Err(error) = validate(&run, &plan.cli, selection) {
            result.rejections.push(error);
            return Ok(result);
        }
        let root = canonical_folder(&run.repository_root).await?;
        // Preserve supplied names as well as resolved host identity: native volume names use basenames.
        let resolved_extra_data_folders = canonical_folders(&plan.extra_data_folders).await?;
        let resolved_passthrough_folders = canonical_folders(&run.passthrough_folders).await?;
        let context = DiscoverRequest {
            project_root: root.clone(),
            cli: plan.cli.clone(),
        };
        let mut discover = match &self.shutdown {
            Some(shutdown) => DiscoverApplications::with_shutdown(shutdown.clone()),
            None => DiscoverApplications::default(),
        };
        let catalog = discover.run(context.clone()).await?;
        result.diagnostics = catalog.diagnostics.clone();
        if catalog.outcome != CatalogOutcome::Available as i32 {
            result.rejections.push(format!(
                "Application catalog is unavailable: {}",
                catalog.error
            ));
            return Ok(result);
        }
        let candidates = selected_candidates(&catalog, selection);
        if candidates.is_empty() {
            result.rejections.push("Requested application/language/mode is not present in the native application catalog".into());
            return Ok(result);
        }
        let inputs = match read_project_inputs(Path::new(&root)).await {
            Ok(inputs) => inputs,
            Err(error) => {
                result.rejections.push(error.to_string());
                return Ok(result);
            }
        };
        let observed_metadata = metadata_inputs(&candidates).await?;
        for file in &observed_metadata {
            if file.sha256.is_empty() {
                result.unknown.push(format!(
                    "Native metadata file identity is unavailable at {}",
                    file.path
                ));
            }
        }
        let negotiated = tokio::select! {
            _ = cancelled(&mut self.shutdown) => return Err(Error::message("Closing")),
            observed = self.engine.negotiate() => observed,
        };
        if let Err(error) = negotiated {
            result
                .rejections
                .push(format!("Console Docker API negotiation failed: {error}"));
            return Ok(result);
        }
        let daemon = tokio::select! {
            _ = cancelled(&mut self.shutdown) => return Err(Error::message("Closing")),
            observed = verify_native_daemon(Path::new(&root), &self.engine) => observed,
        };
        let daemon = match daemon {
            Ok(id) => id,
            Err(error) => {
                result
                    .rejections
                    .push(format!("Native Docker coherence failed: {error}"));
                return Ok(result);
            }
        };
        let image_id = self
            .observe_image(&request.build_record, &mut result)
            .await?;
        let mut args = vec!["run", selection.application.as_str()];
        if !selection.mode.is_empty() {
            args.push(&selection.mode);
        }
        let language = normalize_language(&selection.language);
        if !language.is_empty() {
            args.extend(["--language", &language]);
        }
        args.extend([
            "--no-docker-build",
            "--no-local-build",
            "--persistent",
            "--dryrun",
        ]);
        for folder in &plan.extra_data_folders {
            args.extend(["--add-volume", folder]);
        }
        let diagnostic = query(&context, &args, &mut self.shutdown).await?;
        if diagnostic.termination != Some(Termination::ExitCode(0)) || !diagnostic.error.is_empty()
        {
            result
                .rejections
                .push("Native dry run failed; see retained stdout/stderr and termination".into());
        }
        result.diagnostics.push(diagnostic);
        if let Some(build) = &result.build {
            if matches!(
                BuildState::try_from(build.state),
                Ok(BuildState::NotBuilt | BuildState::RebuildNeeded)
            ) {
                result.rejections.extend(build.diagnostics.clone());
            }
        }
        // Reject changes to the observations while native queries were running.
        let unchanged = async {
            Ok::<_, Error>(
                read_project_inputs(Path::new(&root)).await? == inputs
                    && metadata_inputs(&candidates).await? == observed_metadata
                    && canonical_folders(&plan.extra_data_folders).await?
                        == resolved_extra_data_folders
                    && canonical_folders(&run.passthrough_folders).await?
                        == resolved_passthrough_folders,
            )
        }
        .await;
        match unchanged {
            Ok(true) => {},
            Ok(false) => result.rejections.push("Observed native input files or folder identities changed during Review; repeat Review".into()),
            Err(error) => result.rejections.push(format!("Cannot recheck observed Review inputs: {error}")),
        }
        let mut snapshot = ConfigurationSnapshot {
            selection: Some(selection.clone()),
            image_id,
            project_inputs: Some(inputs),
            repository_root: root,
            cli: plan.cli,
            observed_metadata,
            daemon_id: daemon,
            host_shm_access: run.host_shm_access,
            write_policy: run.write_policy,
            extra_data_folders: plan.extra_data_folders,
            passthrough_folders: run.passthrough_folders,
            resolved_extra_data_folders,
            resolved_passthrough_folders,
            catalog_sha256: selected_query_digest(&catalog, selection)?,
            readiness_seconds: run.readiness_seconds,
            stop_seconds: run.stop_seconds,
            current_source: request.current_source,
            build_record_sha256: request
                .build_record
                .as_ref()
                .map(|record| host_filesystem::files::digest(&record.encode_to_vec()))
                .unwrap_or_default(),
            ..Default::default()
        };
        snapshot.sha256 = host_filesystem::files::digest(&snapshot.encode_to_vec());
        result.snapshot = Some(snapshot);
        Ok(result)
    }
}

impl PrepareLaunch {
    async fn observe_image(
        &mut self,
        record: &Option<crate::BuildRecord>,
        result: &mut Preparation,
    ) -> Result<String> {
        let Some(record) = record.as_ref().filter(|record| !record.image_id.is_empty()) else {
            return Ok(String::new());
        };
        let observed = tokio::select! {
            _ = cancelled(&mut self.shutdown) => return Err(Error::message("Closing")),
            observed = self.engine.image_id(&record.image_id) => observed,
        };
        let mut observation = RecordedImageObservation {
            recorded_id: record.image_id.clone(),
            ..Default::default()
        };
        let image_id = match observed {
            Ok(Some(id)) => {
                observation.observed_id = id.clone();
                observation.state = if id == record.image_id {
                    RecordedImageState::Present
                } else {
                    if let Some(build) = &mut result.build {
                        build.state = BuildState::RebuildNeeded as i32;
                        build.diagnostics.push(
                            "Recorded Build image identity differs from the Engine observation"
                                .into(),
                        );
                    }
                    RecordedImageState::Mismatch
                } as i32;
                id
            }
            Ok(None) => {
                observation.state = RecordedImageState::Missing as i32;
                if let Some(build) = &mut result.build {
                    build.state = BuildState::NotBuilt as i32;
                    build
                        .diagnostics
                        .push("Recorded Build image is missing from the selected Engine".into());
                }
                String::new()
            }
            Err(error) => {
                observation.state = RecordedImageState::Failed as i32;
                observation.error = error.to_string();
                result
                    .rejections
                    .push(format!("Recorded Build image observation failed: {error}"));
                String::new()
            }
        };
        result.recorded_image = Some(observation);
        Ok(image_id)
    }
}
fn validate(run: &RunPlan, cli: &str, selection: &Selection) -> std::result::Result<(), String> {
    if !run.system_apps.is_empty() || !run.dora_apps.is_empty() || !run.mcap_apps.is_empty() {
        return Err(
            "Holoscan system-app and recording-writer integration is not available yet".into(),
        );
    }
    workload_lifecycle::plan::validate_apps(run, std::iter::empty())?;
    if run.host_shm_access != HostShmAccess::Passthrough as i32 {
        return Err("Holoscan Review requires explicit HOST_SHM_ACCESS_PASSTHROUGH; unknown native Docker controls cannot prove BLOCKED".into());
    }
    if !run.recordings.is_empty() {
        return Err("Holoscan recording integration is not available yet".into());
    }
    if !Path::new(cli).is_absolute()
        || !Path::new(&run.repository_root).is_absolute()
        || selection.application.is_empty()
    {
        return Err(
            "Holoscan cli/repository_root must be absolute and application must be selected".into(),
        );
    }
    if run.readiness_seconds == 0 || run.stop_seconds == 0 {
        return Err("readiness_seconds and stop_seconds must be positive".into());
    }
    Ok(())
}
async fn canonical_folder(path: &str) -> Result<String> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err(Error::Configuration(
            "Holoscan folders must be absolute".into(),
        ));
    }
    let actual = tokio::fs::canonicalize(path)
        .await
        .map_err(|error| Error::message(format!("cannot resolve {}: {error}", path.display())))?;
    if !tokio::fs::metadata(&actual)
        .await
        .map_err(|error| Error::message(error.to_string()))?
        .is_dir()
    {
        return Err(Error::Configuration(format!(
            "{} must be a directory",
            path.display()
        )));
    }
    actual
        .into_os_string()
        .into_string()
        .map_err(|_| Error::Configuration("Holoscan folders must be UTF-8".into()))
}
async fn canonical_folders(paths: &[String]) -> Result<Vec<String>> {
    let mut actual = Vec::new();
    for path in paths {
        actual.push(canonical_folder(path).await?);
    }
    Ok(actual)
}
async fn metadata_inputs(folders: &[&str]) -> Result<Vec<NativeInputFile>> {
    let mut files = BTreeMap::new();
    for folder in folders {
        let path = Path::new(folder).join("metadata.json");
        let path = path
            .to_str()
            .ok_or_else(|| Error::Configuration("Native metadata paths must be UTF-8".into()))?;
        let digest = match tokio::fs::read(path).await {
            Ok(bytes) => host_filesystem::files::digest(&bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => {
                return Err(Error::message(format!(
                    "cannot read observed metadata {path}: {error}"
                )));
            }
        };
        files.insert(path.to_owned(), digest);
    }
    Ok(files
        .into_iter()
        .map(|(path, sha256)| NativeInputFile { path, sha256 })
        .collect())
}
fn selected_candidates<'a>(catalog: &'a Catalog, selection: &Selection) -> Vec<&'a str> {
    let language = normalize_language(&selection.language);
    catalog
        .entries
        .iter()
        .filter(|entry| entry.project_type == "application" && entry.name == selection.application)
        .filter(|entry| {
            entry.variants.iter().any(|variant| {
                (language.is_empty() || normalize_language(&variant.language) == language)
                    && (selection.mode.is_empty()
                        || variant.modes.iter().any(|mode| mode.name == selection.mode))
            })
        })
        .map(|entry| entry.source_folder.as_str())
        .collect()
}
fn selected_query_digest(catalog: &Catalog, selection: &Selection) -> Result<String> {
    let mut payloads = Vec::new();
    for diagnostic in &catalog.diagnostics {
        if diagnostic.argv.get(3).is_some_and(|arg| arg == "modes")
            && diagnostic.argv.get(4) == Some(&selection.application)
        {
            let value: serde_json::Value = serde_json::from_slice(&diagnostic.stdout)
                .map_err(|error| Error::message(error.to_string()))?;
            payloads.push(value);
        }
    }
    let bytes = serde_json::to_vec(&payloads).map_err(|error| Error::message(error.to_string()))?;
    Ok(host_filesystem::files::digest(&bytes))
}
