//! Resolve a review using the same adapter and supporting-app template as execution.
use crate::{
    policy, project,
    protocol::{ConsoleConfig, PrepareRequest, Review, Service},
};
use std::{
    collections::HashSet,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    rc::Rc,
};
use workload_docker::{
    compose::{ComposeProject, DockerAdapter},
    workflows::recorded_compose::{
        ComposeRun,
        resolution::{self, ReviewReceipt},
    },
};

/// Private inputs retained until the review is consumed or replaced.
pub struct PreparedReview {
    pub review: Review,
    pub run: ComposeRun,
    pub receipt: ReviewReceipt,
}

impl PreparedReview {
    /// Console prepares exactly one workload alongside optional supporting apps.
    pub fn project(&self) -> &ComposeProject {
        &self.run.projects[0]
    }
}

/// A launchable review owns private inputs; a profile prompt owns only display data.
pub enum Preparation {
    Launchable(Box<PreparedReview>),
    NeedsProfiles(Review),
}
impl Preparation {
    /// Consume a launchable result; profile prompts have no retained launch inputs.
    pub fn into_launchable(self) -> Option<PreparedReview> {
        match self {
            Self::Launchable(prepared) => Some(*prepared),
            Self::NeedsProfiles(_) => None,
        }
    }
}

/// Resolve local image identities and private manifests without launching resources.
pub async fn prepare(
    adapter: Rc<DockerAdapter>,
    config: &ConsoleConfig,
    request: PrepareRequest,
) -> Result<Preparation, String> {
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
    let mut random = [0_u8; policy::RANDOM_ID_BYTES];
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
        Err(workload_docker::compose::Error::EmptySelection)
            if !available.is_empty() && project.profiles.is_empty() =>
        {
            let _ = std::fs::remove_dir_all(&directory);
            Ok(Preparation::NeedsProfiles(Review {
                available_profiles: available,
                warnings: vec!["Select a profile to enable at least one service.".into()],
                system_apps: crate::system_apps::options(config, &apps),
                ..Default::default()
            }))
        }
        Err(error) => {
            let _ = std::fs::remove_dir_all(&directory);
            Err(error.to_string())
        }
        Ok(review) => Ok(Preparation::Launchable(Box::new(review))),
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
) -> workload_docker::compose::Result<PreparedReview> {
    let runtime = crate::config::runtime(config);
    let run = ComposeRun {
        filesystem: Some(Default::default()),
        id: run_id.clone(),
        artifacts: directory,
        recovery_anchor_dir: None,
        projects: vec![project.clone()],
        first_party: vec![],
        system_apps: apps.clone(),
        readiness_seconds: u64::from(runtime.readiness_seconds.unwrap()),
        stop_seconds: u64::from(runtime.stop_seconds.unwrap()),
    };
    let resolved = resolution::preview(adapter, &run).await?;
    let workload = resolved
        .groups
        .iter()
        .flat_map(|group| &group.projects)
        .find(|plan| plan.key == project.key)
        .ok_or_else(|| {
            workload_docker::compose::Error::Invalid("workload project missing".into())
        })?;
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
        run,
        receipt: resolved.receipt,
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
    let mut random = [0_u8; policy::RANDOM_ID_BYTES];
    if getrandom::fill(&mut random).is_err() {
        return;
    }
    let path = directory.join(format!("prepare-{}.json", hex::encode(random)));
    let _ = workload_docker::evidence::write_private_once(
        &path,
        &serde_json::json!({ "operation": "prepare", "error": message }),
    );
}
