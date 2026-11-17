//! Partial native Review: verified inputs and recorded artifacts, with unavailable launch details explicit.
use crate::{
    BuildState, Catalog, CatalogOutcome, ConfigurationSnapshot, DiscoverApplications,
    DiscoverRequest, NativeInputFile, Preparation, PrepareRequest, RecordedImageObservation,
    RecordedImageState, Selection,
    build_record::assess_build_record,
    discovery::{cancelled, container_query, normalize_language},
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

/// Review native input/Build facts and resolve actual Compose system applications.
/// Native defaults and Docker controls remain partial observations; explicit PASSTHROUGH is required.
/// System resolution writes private artifacts; no containers or filesystem views start.
///
/// # Ownership
/// The caller scopes Build evidence to the admitted canonical repository. This Step retains actual
/// system resolution in its Step output for one subsequent prepare_resolved call.
///
/// # Cancellation
/// Native queries are killed and reaped. Attempted system resolution settles before shutdown returns.
/// Owners await completion before closing the long-lived runtime; private artifacts remain retained.
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
    type Output = (Preparation, Option<compose_apps::preparation::ResolvedRun>);

    async fn run(&mut self, (run, request): (RunPlan, PrepareRequest)) -> Result<Self::Output> {
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
        let mut system_input = match validate(&run, &plan.cli, selection, &plan.system_projects) {
            Ok(input) => input,
            Err(error) => {
                result.rejections.push(error);
                return Ok((result, None));
            }
        };
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
            return Ok((result, None));
        }
        let candidates = selected_candidates(&catalog, selection);
        if candidates.is_empty() {
            result.rejections.push("Requested application/language/mode is not present in the native application catalog".into());
            return Ok((result, None));
        }
        let inputs = match read_project_inputs(Path::new(&root)).await {
            Ok(inputs) => inputs,
            Err(error) => {
                result.rejections.push(error.to_string());
                return Ok((result, None));
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
            return Ok((result, None));
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
                return Ok((result, None));
            }
        };
        let systems = if plan.system_projects.is_empty() {
            None
        } else {
            system_input.run_plan.repository_root = root.clone();
            let adapter = compose_apps::compose::DockerAdapter::from_engine(self.engine.clone());
            // Shared resolution may write private artifacts; settle it before honoring shutdown.
            let resolved = compose_apps::preparation::resolve(&adapter, system_input).await;
            if self
                .shutdown
                .as_ref()
                .is_some_and(|shutdown| *shutdown.borrow() || shutdown.has_changed().is_err())
            {
                return Err(Error::message("Closing"));
            }
            match resolved {
                Ok(resolved) => {
                    result.warnings.extend(resolved.warnings.clone());
                    Some(resolved)
                }
                Err(error) => {
                    result
                        .rejections
                        .push(format!("System application preparation failed: {error}"));
                    return Ok((result, None));
                }
            }
        };
        if let Some(build) = &result.build
            && matches!(
                BuildState::try_from(build.state),
                Ok(BuildState::NotBuilt | BuildState::RebuildNeeded)
            )
        {
            result.rejections.extend(build.diagnostics.clone());
        }
        let image_id = self
            .observe_image(&request.build_record, &mut result)
            .await?;
        let mut args = vec!["run"];
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
        args.extend(["--", selection.application.as_str()]);
        if !selection.mode.is_empty() {
            args.push(&selection.mode);
        }
        let diagnostic = container_query(&context, &args, &mut self.shutdown).await?;
        if diagnostic.termination != Some(Termination::ExitCode(0)) || !diagnostic.error.is_empty()
        {
            result
                .rejections
                .push("Native dry run failed; see retained stdout/stderr and termination".into());
        }
        result.diagnostics.push(diagnostic);
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
            system_configuration: systems.as_ref().map(|resolved| resolved.snapshot.clone()),
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
        Ok((result, systems))
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
                    let cause = "Recorded Build image identity differs from the Engine observation";
                    if let Some(build) = &mut result.build {
                        build.state = BuildState::RebuildNeeded as i32;
                        build.diagnostics.push(cause.into());
                    }
                    result.rejections.push(cause.into());
                    RecordedImageState::Mismatch
                } as i32;
                id
            }
            Ok(None) => {
                observation.state = RecordedImageState::Missing as i32;
                let cause = "Recorded Build image is missing from the selected Engine";
                if let Some(build) = &mut result.build {
                    build.state = BuildState::NotBuilt as i32;
                    build.diagnostics.push(cause.into());
                }
                result.rejections.push(cause.into());
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
pub(crate) fn validate(
    run: &RunPlan,
    cli: &str,
    selection: &Selection,
    projects: &[compose_apps::compose::ComposeProject],
) -> std::result::Result<compose_apps::preparation::PrepareInput, String> {
    let input = crate::system::compose_input(run.clone(), projects.to_vec())
        .map_err(|error| error.to_string())?;
    if !run.recordings.is_empty() && projects.is_empty() {
        return Err("Holoscan recordings require configured system applications".into());
    }
    if run.host_shm_access != HostShmAccess::Passthrough as i32 {
        return Err("Holoscan Review requires explicit HOST_SHM_ACCESS_PASSTHROUGH; unknown native Docker controls cannot prove BLOCKED".into());
    }
    if !Path::new(cli).is_absolute()
        || !Path::new(&run.repository_root).is_absolute()
        || selection.application.is_empty()
    {
        return Err(
            "Holoscan cli/repository_root must be absolute and application must be selected".into(),
        );
    }
    Ok(input)
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
        if diagnostic.argv.get(3).is_some_and(|arg| arg == "modes") {
            let value: serde_json::Value = serde_json::from_slice(&diagnostic.stdout)
                .map_err(|error| Error::message(error.to_string()))?;
            if value["project"].as_str() == Some(selection.application.as_str()) {
                payloads.push(value);
            }
        }
    }
    let bytes = serde_json::to_vec(&payloads).map_err(|error| Error::message(error.to_string()))?;
    Ok(host_filesystem::files::digest(&bytes))
}

#[cfg(test)]
mod tests;
