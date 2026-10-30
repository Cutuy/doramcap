//! Resolve a review using the same adapter and supporting-app template as execution.
use crate::{
    project,
    protocol::{ConsoleConfig, PrepareRequest, Review, Service},
};
use orchestrator::Step;
use std::{
    collections::HashSet,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    rc::Rc,
};
use workload_docker::{
    compose::{ComposeProject, DockerAdapter, ResolveInput},
    workflows::system_apps::{PrepareInput, PrepareSystemApps},
};

/// Private inputs retained until the review is consumed or replaced.
pub struct PreparedReview {
    pub review: Review,
    pub project: ComposeProject,
    pub workload_digest: String,
    pub monitor_image: Option<String>,
    pub system_apps: workload_docker::workflows::system_apps::SystemAppsConfig,
    pub directory: PathBuf,
}

/// Resolve local image identities and private manifests without launching resources.
pub async fn prepare(
    adapter: Rc<DockerAdapter>,
    config: &ConsoleConfig,
    request: PrepareRequest,
) -> Result<PreparedReview, String> {
    let apps = crate::system_apps::select(config, request.system_apps.as_ref())?;
    let project = project::discover(
        Path::new(&config.project_root),
        Path::new(&request.directory),
        request.profiles,
    )?;
    let available = adapter
        .profiles(&project)
        .await
        .map_err(|e| e.to_string())?;
    let allowed: HashSet<_> = available.iter().collect();
    let mut selected = HashSet::new();
    if project
        .profiles
        .iter()
        .any(|p| !allowed.contains(p) || !selected.insert(p))
    {
        return Err("unknown or duplicate profile".into());
    }
    adapter
        .verify_overlay_namespace()
        .await
        .map_err(|e| e.to_string())?;
    let mut random = [0_u8; 16];
    getrandom::fill(&mut random).map_err(|e| e.to_string())?;
    let run_id = hex::encode(random);
    let reviews = Path::new(&config.data_root).join("reviews");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&reviews)
        .map_err(|e| e.to_string())?;
    let directory = reviews.join(&run_id);
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .map_err(|e| e.to_string())?;
    let result = resolve(
        adapter,
        config,
        project.clone(),
        available.clone(),
        run_id,
        directory.clone(),
        apps.clone(),
    )
    .await;
    match result {
        Err(error)
            if error == "selection contains no services"
                && !available.is_empty()
                && project.profiles.is_empty() =>
        {
            let _ = std::fs::remove_dir_all(&directory);
            Ok(PreparedReview {
                review: Review {
                    prepared_id: String::new(),
                    run_id: String::new(),
                    available_profiles: available,
                    selected_profiles: Vec::new(),
                    services: Vec::new(),
                    warnings: vec!["Select a profile to enable at least one service.".into()],
                    system_apps: crate::system_apps::options(config, &apps),
                },
                project,
                workload_digest: String::new(),
                monitor_image: None,
                system_apps: apps,
                directory,
            })
        }
        Err(error) => {
            let _ = std::fs::remove_dir_all(&directory);
            Err(error)
        }
        Ok(review) => Ok(review),
    }
}

async fn resolve(
    adapter: Rc<DockerAdapter>,
    config: &ConsoleConfig,
    project: ComposeProject,
    available: Vec<String>,
    run_id: String,
    directory: PathBuf,
    apps: workload_docker::workflows::system_apps::SystemAppsConfig,
) -> Result<PreparedReview, String> {
    let workload = adapter
        .resolve(ResolveInput {
            run: run_id.clone(),
            artifacts: directory.clone(),
            project: project.clone(),
        })
        .await
        .map_err(|e| e.to_string())?;
    let system = PrepareSystemApps
        .run(PrepareInput {
            config: apps.clone(),
            artifacts: directory.clone(),
        })
        .await
        .map_err(|e| e.to_string())?;
    let monitor_image = if let Some(monitor_project) = system.projects.into_iter().next() {
        let monitor = adapter
            .resolve(ResolveInput {
                run: run_id.clone(),
                artifacts: directory.clone(),
                project: monitor_project,
            })
            .await
            .map_err(|e| e.to_string())?;
        Some(
            monitor
                .services
                .get("cpu-monitor")
                .ok_or("CPU monitor service missing")?
                .image_id
                .clone(),
        )
    } else {
        None
    };
    let services = workload
        .services
        .iter()
        .map(|(name, service)| Service {
            name: name.clone(),
            image_id: service.image_id.clone(),
        })
        .collect();
    let review = Review {
        prepared_id: run_id.clone(),
        run_id,
        available_profiles: available,
        selected_profiles: project.profiles.clone(),
        services,
        // Raw Compose diagnostics stay in private evidence; expose actionable prompts only.
        warnings: Vec::new(),
        system_apps: crate::system_apps::options(config, &apps),
    };
    Ok(PreparedReview {
        review,
        project,
        workload_digest: workload.sha256,
        monitor_image,
        system_apps: apps,
        directory,
    })
}

/// Save a detailed diagnostic in the private data root, never in an HTTP response.
pub(crate) fn save_diagnostic(config: &ConsoleConfig, message: &str) {
    let directory = Path::new(&config.data_root).join("diagnostics");
    if std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&directory)
        .is_err()
    {
        return;
    }
    let mut random = [0_u8; 16];
    if getrandom::fill(&mut random).is_err() {
        return;
    }
    let path = directory.join(format!("prepare-{}.json", hex::encode(random)));
    let _ = workload_docker::evidence::write_private_once(
        &path,
        &serde_json::json!({ "operation": "prepare", "error": message }),
    );
}
