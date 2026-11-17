//! Concrete shared system-app ownership beside the retained native application.
use compose_apps::{
    compose::{
        ComposeProjectState, Diagnostics, DockerAdapter, PreparedOverlays, ResolvedComposeProject,
    },
    experiment::{ExperimentOwner, SharedExperiment},
    preparation::{PreparedRun, ResolvedRun},
};
use host_filesystem::{changes::SourcesInput, overlay::OverlaySet};
use orchestrator::{LocalExecutor, Output, Step};
use std::{rc::Rc, time::Duration};
use tokio::sync::{Mutex, watch};

/// Prepare native bind paths/endpoints from the one reviewed system resolution.
/// The owning workflow compares Review first and materializes shared sources later.
pub(crate) struct PrepareSystems;
impl Step for PrepareSystems {
    const ID: Option<&'static str> = Some("holoscan.prepare-systems");
    const DISPLAY_NAME: Option<&'static str> = Some("Prepare Holoscan system applications");
    type Input = ResolvedRun;
    type Output = PreparedRun;
    async fn run(&mut self, resolved: Self::Input) -> orchestrator::Result<Self::Output> {
        compose_apps::preparation::prepare_resolved(resolved)
            .await
            .map_err(|error| orchestrator::Error::message(error.to_string()))
    }
}

/// Own configured systems before startup; the workflow retains the shared views and finalization.
/// Partial reservation/startup remains Engine-owned and is stopped through this same run.
pub(crate) struct Systems {
    pub prepared: Output<PreparedRun>,
    pub adapter: Rc<DockerAdapter>,
    pub owner: SharedExperiment,
    pub diagnostics: Diagnostics,
    pub plan: ResolvedComposeProject,
    pub last_launch: Option<Output<ComposeProjectState>>,
    pub stopped: Option<compose_apps::system_apps::StoppedSystemApps>,
}
impl Systems {
    /// Bind shared lifecycle/cgroup steps before any system startup await.
    pub fn bind(
        executor: &mut LocalExecutor,
        prepared: Output<PreparedRun>,
        adapter: Rc<DockerAdapter>,
        diagnostics: Diagnostics,
    ) -> Self {
        let plan = prepared.value().plan.clone();
        let run = &prepared.value().run;
        let owner = Rc::new(Mutex::new(ExperimentOwner::new(
            prepared.value().dora.clone(),
            run.clone(),
            adapter.clone(),
            plan.clone(),
            run.readiness_seconds,
            run.stop_seconds,
        )));
        compose_apps::steps::register(executor, adapter.clone(), diagnostics.clone());
        compose_apps::steps::register_experiment(executor, owner.clone());
        if let Some(placement) = &prepared.value().cgroup {
            let group = compose_apps::cgroup::DockerCgroup::group(
                placement.clone(),
                Duration::from_secs(run.stop_seconds),
            );
            workload_lifecycle::cgroup::register(executor, group);
        }
        Self {
            prepared,
            adapter,
            owner,
            diagnostics,
            plan,
            last_launch: None,
            stopped: None,
        }
    }

    /// Use the rewritten plan before reserving or starting resources, then reuse shared startup.
    pub async fn start(
        &mut self,
        executor: &LocalExecutor,
        overlays: &Output<PreparedOverlays>,
    ) -> Result<(), String> {
        if overlays.value().projects.len() != 1 {
            return Err("one resolved system project is required".into());
        }
        self.plan = self.prepared.value().execution_plan(Some(overlays.value()));
        let run = &self.prepared.value().run;
        // No system startup has run yet; bind the execution view before its first side effect.
        *self.owner.lock().await = ExperimentOwner::new(
            self.prepared.value().dora.clone(),
            run.clone(),
            self.adapter.clone(),
            self.plan.clone(),
            run.readiness_seconds,
            run.stop_seconds,
        );
        if self.prepared.value().dora.enabled() {
            self.adapter
                .project(&self.plan)
                .reserve(
                    self.prepared.value().dora.daemon().map_err(error)?.0,
                    run.readiness_seconds,
                )
                .await
                .map_err(error)?;
        }
        compose_apps::system_apps::start(
            executor,
            &self.prepared,
            &self.plan,
            Some(overlays),
            &mut self.last_launch,
            &self.adapter,
        )
        .await
        .map_err(error)?;
        compose_apps::system_apps::verify_stable(
            executor,
            &self.plan,
            &self.prepared.value().system_services,
        )
        .await
        .map_err(error)
    }

    /// Observe systems from the one run event capture; callers observe event errors separately.
    pub async fn watch(
        &self,
        executor: &LocalExecutor,
        changes: watch::Receiver<Result<(), String>>,
    ) -> Result<(), String> {
        compose_apps::system_apps::watch(
            executor,
            &self.plan,
            &self.prepared.value().system_services,
            &self.owner,
            changes,
        )
        .await
        .map_err(error)
    }

    /// Stop systems/Dora after native-only stop, bounded by the same whole-run deadline.
    pub async fn stop(
        &mut self,
        executor: &LocalExecutor,
        deadline: tokio::time::Instant,
    ) -> Result<(), String> {
        // Startup can create containers after cancellation; settle it even when native stop spent the budget.
        let startup = self
            .adapter
            .project(&self.plan)
            .settle_start_evidence()
            .await;
        self.diagnostics.borrow_mut().extend(startup);
        if tokio::time::Instant::now() >= deadline {
            let error = "stop deadline expired while settling finite Compose startup";
            self.diagnostics.borrow_mut().push(
                compose_apps::compose::error::invalid(error)
                    .diagnostic("Compose startup settlement"),
            );
            return Err(error.into());
        }
        let stopped = tokio::time::timeout_at(
            deadline,
            compose_apps::system_apps::stop(
                executor,
                &self.plan,
                &self.prepared.value().system_services,
                &self.owner,
                &self.diagnostics,
            ),
        )
        .await
        .map_err(|_| "system-app stop deadline expired".to_owned())?;
        self.stopped = Some(stopped);
        Ok(())
    }

    /// Preserve system reporting consumers/scopes while attaching the canonical shared mapping identity.
    /// Missing or ambiguous service+target associations fail before source materialization.
    pub fn source_input(&self, views: &OverlaySet) -> Result<SourcesInput, String> {
        let mut input = self.prepared.value().source_input.clone();
        for bind in &mut input.bind_mounts {
            let service = bind
                .consumer
                .strip_prefix(&format!("{}/", self.plan.name))
                .ok_or("system source consumer differs from its resolved project")?;
            let consumer = format!("{}/{service}", self.plan.key);
            let mut matches = views
                .mappings
                .iter()
                .filter(|mapping| mapping.consumer == consumer && mapping.target == bind.target);
            let mapping = matches.next().ok_or("system source mapping is missing")?;
            if matches.next().is_some() {
                return Err("system source mapping is ambiguous".into());
            }
            bind.source = mapping
                .original
                .to_str()
                .ok_or("canonical system source must be UTF-8")?
                .into();
        }
        Ok(input)
    }

    /// Observe owned system container layers after whole-run writer verification.
    pub async fn containers(
        &self,
    ) -> Result<Vec<docker_engine::docker_proto::OwnedContainer>, String> {
        let state = self
            .adapter
            .project(&self.plan)
            .inspect()
            .await
            .map_err(error)?;
        let scopes = self.prepared.value().file_change_scopes();
        state
            .containers
            .into_iter()
            .map(|container| {
                let scope = scopes
                    .get(&container.service)
                    .ok_or("unclassified system container")?;
                Ok(docker_engine::docker_proto::OwnedContainer {
                    id: container.id,
                    consumer: format!("{}/{}", self.plan.name, container.service),
                    app_scope: *scope as i32,
                })
            })
            .collect()
    }
}
fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, os::unix::net::UnixListener};
    use workload_lifecycle::launch::RunPlan;

    struct Prepared;
    impl Step for Prepared {
        type Input = PreparedRun;
        type Output = PreparedRun;
        async fn run(&mut self, input: Self::Input) -> orchestrator::Result<Self::Output> {
            Ok(input)
        }
    }

    #[tokio::test]
    async fn already_expired_native_stop_budget_cannot_start_system_cleanup_or_acknowledge_stopped()
    {
        // Arrange: an actual socket would observe any subsequent Engine request.
        let root = std::env::temp_dir().join(format!(
            "holoscan-stop-budget-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let socket = root.join("engine.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        listener.set_nonblocking(true).unwrap();
        let adapter = Rc::new(DockerAdapter::from_engine(
            docker_engine::Engine::new(socket.to_str().unwrap(), 1).unwrap(),
        ));
        let mut executor = LocalExecutor::default();
        executor.register::<Prepared>(|| Prepared);
        let prepared = executor
            .run::<Prepared>(PreparedRun {
                run: RunPlan {
                    readiness_seconds: 1,
                    stop_seconds: 1,
                    ..Default::default()
                },
                artifacts: root.clone(),
                plan: ResolvedComposeProject::default(),
                cgroup: None,
                system_services: Vec::new(),
                user_services: Vec::new(),
                mcap_services: Vec::new(),
                source_input: SourcesInput {
                    passthrough_folders: Vec::new(),
                    excluded: Vec::new(),
                    overlays: true,
                    bind_mounts: Vec::new(),
                    metadata_sources: Vec::new(),
                    artifacts: root.clone(),
                },
                dora: Default::default(),
                snapshot: Default::default(),
            })
            .await
            .unwrap();
        let mut systems = Systems::bind(
            &mut executor,
            prepared,
            adapter,
            Rc::new(RefCell::new(Vec::new())),
        );
        // Act: native-only stop already exhausted the single run budget.
        let result = systems.stop(&executor, tokio::time::Instant::now()).await;
        // Assert: settlement precedes deadline refusal; no new command/Engine request or false stop fact.
        assert!(result.unwrap_err().contains("deadline expired"));
        assert!(systems.stopped.is_none());
        assert!(
            systems
                .diagnostics
                .borrow()
                .iter()
                .any(|diagnostic| diagnostic.operation == "Compose startup settlement")
        );
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
