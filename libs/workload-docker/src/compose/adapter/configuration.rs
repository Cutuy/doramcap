//! Native Compose/Engine I/O and artifact publication for the pure TypeScript planner.
use super::{DockerAdapter, digest, private_write, project_name, validate_name};
use crate::compose::{
    ComposeProject, ResolveInput, ResolvedComposeProject, Result,
    command::compose,
    error::{absent, engine, invalid},
    planner::{self, *},
};
use prost::Message;
use serde_json::json;
use std::{collections::BTreeMap, path::Path, time::Duration};

/// Native normalization bytes and the source evidence used to produce them.
pub(crate) struct NormalizedCompose {
    pub(crate) model: Vec<u8>,
    pub(crate) compose_version: String,
    pub(crate) input_hashes: BTreeMap<String, String>,
    pub(crate) stderr: String,
}
/// A published Docker manifest and its typed planner description.
pub(crate) struct PlannedCompose {
    pub(crate) project: ResolvedComposeProject,
    pub(crate) configuration: CompleteOutput,
}
/// Runs native normalization, supplies host facts and publishes validated plans.
pub(crate) struct ComposeConfiguration<'a> {
    pub(super) adapter: &'a DockerAdapter,
}
impl ComposeConfiguration<'_> {
    /// Render Compose inputs and collect the CLI version, source-file hashes and stderr.
    /// No runtime resources or manifests are created. Relative paths use the input directory.
    /// Invalid settings, missing files or Compose failures return errors.
    pub(crate) async fn normalize(
        &self,
        run: &str,
        project: &ComposeProject,
        name: &str,
    ) -> Result<NormalizedCompose> {
        validate_name(run)?;
        validate_name(&project.key)?;
        let (directory, mut args, sources) = config_arguments(project, name).await?;
        args.extend(["config".into(), "--format".into(), "json".into()]);
        args.extend(project.services.clone());
        let (stdout, stderr) = compose(
            &self.adapter.socket,
            &directory,
            &project.environment,
            &args,
            Duration::from_secs(60),
        )
        .await?;
        if stderr.contains("variable is not set") {
            return Err(invalid(format!("unresolved Compose environment: {stderr}")));
        }
        let model = stdout.into_bytes();
        let (version, _) = compose(
            &self.adapter.socket,
            &directory,
            &BTreeMap::new(),
            &["version".into(), "--short".into()],
            Duration::from_secs(10),
        )
        .await?;
        Ok(NormalizedCompose {
            model,
            compose_version: version,
            input_hashes: sources,
            stderr,
        })
    }
    /// Resolve one ordinary Compose project through the same typed configuration planner.
    pub(crate) async fn resolve(&self, input: ResolveInput) -> Result<ResolvedComposeProject> {
        validate_name(&input.run)?;
        let project = input
            .project
            .as_ref()
            .ok_or_else(|| invalid("Compose project missing"))?;
        validate_name(&project.key)?;
        let identity = tokio::fs::canonicalize(&project.directory).await?;
        let name = project_name(&project.key, &identity);
        let normalized = self.normalize(&input.run, project, &name).await?;
        let artifacts = tokio::fs::canonicalize(&input.artifacts).await?;
        let mut planned = self
            .plan(
                AnalyzeInput {
                    run: input.run,
                    project_name: name,
                    artifacts: artifacts
                        .to_str()
                        .ok_or_else(|| invalid("artifact path is not UTF-8"))?
                        .into(),
                    adopted_apps: vec![ComposeDocument {
                        settings: Some(project.clone()),
                        compose_json: normalized.model,
                    }],
                    ..Default::default()
                },
                &project.key,
            )
            .await?;
        if !normalized.stderr.trim().is_empty() {
            planned.project.diagnostics.push(normalized.stderr.clone());
        }
        private_write(
            &artifacts.join(format!("{}.resolution.json", project.key)),
            &serde_json::to_vec_pretty(&json!({
                "compose_version": normalized.compose_version.trim(),
                "sources": normalized.input_hashes,
                "request": project,
                "plan": planned.project,
                "diagnostics": normalized.stderr,
            }))?,
        )
        .await?;
        Ok(planned.project)
    }

    /// Collect requested facts, execute the pure planner and publish its validated manifest.
    /// Docker resources remain unstarted; incomplete facts and ownership conflicts fail.
    pub(crate) async fn plan(&self, input: AnalyzeInput, key: &str) -> Result<PlannedCompose> {
        let artifacts = Path::new(&input.artifacts);
        let required = planner::analyze(&input).await?;
        // Host operations consume explicit requirements without traversing Compose documents.
        let mut facts = CompleteInput {
            input: Some(input.clone()),
            ..Default::default()
        };
        for reference in required.image_references {
            let image = self
                .adapter
                .docker
                .inspect_image(&reference)
                .await
                .map_err(|e| engine(format!("inspect image {reference}"), e))?;
            facts.images.push(ImageFact {
                reference,
                image_id: image.id.ok_or_else(|| invalid("Engine omitted image ID"))?,
                declared_volumes: image
                    .config
                    .and_then(|c| c.volumes)
                    .map(|v| v.into_iter().collect())
                    .unwrap_or_default(),
            });
        }
        let directory = artifacts.join("inputs");
        crate::evidence::create_private_dir(&directory).await?;
        for source in required.input_sources {
            facts
                .sources
                .push(freeze_source(&source, &directory).await?);
        }
        for name in required.container_names {
            match self.adapter.docker.inspect_container(&name, None).await {
                Ok(_) => return Err(invalid(format!("container name already occupied: {name}"))),
                Err(e) if absent(&e) => facts.available_container_names.push(name),
                Err(e) => return Err(engine("check fixed container name", e)),
            }
        }
        // The result is generated protobuf; TypeScript also owns the Docker JSON projection.
        let configuration = planner::complete(facts).await?;
        let model = configuration
            .compose
            .as_ref()
            .ok_or_else(|| invalid("planner omitted Compose model"))?;
        let bytes = planner::render(model).await?;
        let project = ResolvedComposeProject {
            run: input.run.clone(),
            key: key.into(),
            name: input.project_name.clone(),
            manifest: artifacts
                .join(format!("{key}.compose.json"))
                .to_string_lossy()
                .into_owned(),
            sha256: digest(&bytes),
            services: configuration.services.clone(),
            diagnostics: configuration
                .diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect(),
            remove_volumes: configuration.remove_volumes,
        };
        let execution = self.adapter.project(&project);
        if !execution.containers().await?.is_empty()
            || !execution.networks().await?.is_empty()
            || !execution.volumes().await?.is_empty()
        {
            return Err(invalid("runtime project resources already exist"));
        }
        // Publish private evidence before native schema validation; the caller still owns cleanup.
        private_write(Path::new(&project.manifest), &bytes).await?;
        private_write(
            &artifacts.join(format!("{key}.planner-input.pb")),
            &input.encode_to_vec(),
        )
        .await?;
        private_write(
            &artifacts.join(format!("{key}.planner-output.pb")),
            &configuration.encode_to_vec(),
        )
        .await?;
        execution.validate_execution().await?;
        Ok(PlannedCompose {
            project,
            configuration,
        })
    }
}

/// Snapshot a read-only file with its permissions; directory binds retain their original source.
pub(super) async fn freeze_source(source: &str, directory: &Path) -> Result<InputSourceFact> {
    let canonical = tokio::fs::canonicalize(source).await?;
    let metadata = tokio::fs::metadata(&canonical).await?;
    let (kind, effective_source) = if metadata.is_dir() {
        (input_source_fact::Kind::Directory, source.to_owned())
    } else {
        use std::os::unix::fs::PermissionsExt;
        let bytes = crate::evidence::read_regular(&canonical, 256 * 1024 * 1024).await?;
        let permissions = metadata.permissions();
        let destination = directory.join(format!("{}-{:o}", digest(&bytes), permissions.mode()));
        if !destination.exists() {
            private_write(&destination, &bytes).await?;
            tokio::fs::set_permissions(&destination, permissions).await?;
        }
        (
            input_source_fact::Kind::File,
            destination.to_string_lossy().into_owned(),
        )
    };
    Ok(InputSourceFact {
        source: source.into(),
        kind: kind.into(),
        effective_source,
    })
}
// Shared Compose arguments preserve the explicit environment and input order.
pub(super) async fn config_arguments(
    project: &ComposeProject,
    name: &str,
) -> Result<(std::path::PathBuf, Vec<String>, BTreeMap<String, String>)> {
    if project.files.is_empty() {
        return Err(invalid("at least one Compose file is required"));
    }
    if project
        .environment
        .keys()
        .any(|k| k.starts_with("DOCKER_") || k.starts_with("COMPOSE_"))
    {
        return Err(invalid(
            "Docker/Compose control variables must be supplied as explicit launch settings",
        ));
    }
    let directory = tokio::fs::canonicalize(&project.directory).await?;
    let mut args = vec![
        "--project-directory".into(),
        directory.to_string_lossy().into_owned(),
        "--project-name".into(),
        name.into(),
        "--env-file".into(),
        "/dev/null".into(),
    ];
    let mut sources = BTreeMap::new();
    for (flag, files) in [("-f", &project.files), ("--env-file", &project.env_files)] {
        for file in files {
            let path = tokio::fs::canonicalize(directory.join(file)).await?;
            sources.insert(
                path.to_string_lossy().into_owned(),
                digest(&tokio::fs::read(&path).await?),
            );
            args.extend([flag.into(), path.to_string_lossy().into_owned()]);
        }
    }
    for profile in &project.profiles {
        args.extend(["--profile".into(), profile.clone()]);
    }
    Ok((directory, args, sources))
}
