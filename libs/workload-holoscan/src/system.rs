//! Actual Compose system inputs beside the native Holoscan application.
pub(crate) mod runtime;
use compose_apps::{
    compose::{ComposeProject, Result, error::invalid},
    preparation::{McapWriters, PrepareInput, validate_budgets},
};
use host_filesystem::overlay::{Disposition, OverlayPolicy};
use std::path::Path;
use workload_lifecycle::{
    launch::RunPlan,
    plan::{Phase, phase, validate_apps},
};

/// Preserve actual Compose inputs in the shared Dora/system phases, with no Compose user app.
/// RunPlan owns role IDs, filesystem policy and budgets; this adapter performs no I/O.
/// Empty/duplicate/unknown roles, extra Compose user apps and invalid shared policies fail.
pub(crate) fn compose_input(run: RunPlan, projects: Vec<ComposeProject>) -> Result<PrepareInput> {
    validate_apps(&run, projects.iter().map(|project| project.key.as_str())).map_err(invalid)?;
    validate_budgets(run.readiness_seconds, run.stop_seconds)?;
    let mut dora_apps = Vec::new();
    let mut system_apps = Vec::new();
    for project in projects {
        match phase(&run, &project.key) {
            Phase::Dora => dora_apps.push(project),
            Phase::System => system_apps.push(project),
            Phase::User => {
                return Err(invalid(format!(
                    "{}: Compose user apps are not Holoscan system inputs",
                    project.key
                )));
            }
        }
    }
    Ok(PrepareInput {
        filesystem: Some(OverlayPolicy {
            disposition: Disposition::from_policy(run.write_policy)?,
            passthrough_folders: run.passthrough_folders.clone(),
        }),
        mcap_writers: McapWriters {
            services: Vec::new(),
            apps: run.mcap_apps.clone(),
            directories: Path::new(&run.artifacts)
                .parent()
                .map(Path::to_path_buf)
                .into_iter()
                .collect(),
        },
        run_plan: run,
        dora_apps,
        system_apps,
        extra_apps: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use workload_lifecycle::launch::HostShmAccess;

    fn run() -> RunPlan {
        RunPlan {
            id: "attempt".into(),
            repository_root: "/project".into(),
            artifacts: "/records/attempt".into(),
            readiness_seconds: 30,
            stop_seconds: 10,
            host_shm_access: HostShmAccess::Passthrough as i32,
            ..Default::default()
        }
    }

    #[test]
    fn actual_system_projects_keep_native_inputs_and_shared_policy() {
        // Arrange
        let mut run = run();
        run.dora_apps = vec!["dora-runtime".into()];
        run.system_apps = vec!["monitor".into()];
        run.mcap_apps = vec!["monitor".into()];
        run.passthrough_folders = vec!["/shared".into()];
        let dora = ComposeProject {
            key: "dora-runtime".into(),
            directory: "/apps/dora".into(),
            files: vec!["compose.yml".into()],
            ..Default::default()
        };
        let monitor = ComposeProject {
            key: "monitor".into(),
            directory: "/apps/monitor".into(),
            profiles: vec!["recording".into()],
            environment: BTreeMap::from([("NATIVE_SETTING".into(), "literal $value".into())]),
            ..Default::default()
        };
        // Act
        let input = compose_input(run.clone(), vec![monitor.clone(), dora.clone()]).unwrap();
        // Assert
        assert_eq!(input.run_plan, run);
        assert_eq!(input.dora_apps, vec![dora]);
        assert_eq!(input.system_apps, vec![monitor]);
        assert!(input.extra_apps.is_empty());
        assert_eq!(input.mcap_writers.apps, vec!["monitor"]);
        assert!(input.mcap_writers.services.is_empty());
        assert_eq!(input.mcap_writers.directories, vec![Path::new("/records")]);
        let filesystem = input.filesystem.unwrap();
        assert_eq!(filesystem.disposition, Disposition::WriteBack);
        assert_eq!(filesystem.passthrough_folders, vec!["/shared"]);
    }

    #[test]
    fn external_native_application_needs_no_fabricated_compose_service() {
        // Arrange / Act
        let input = compose_input(run(), vec![]).unwrap();
        // Assert
        assert!(input.dora_apps.is_empty());
        assert!(input.system_apps.is_empty());
        assert!(input.extra_apps.is_empty());
    }

    #[test]
    fn extra_compose_apps_and_invalid_roles_fail_without_side_effects() {
        // Arrange
        let project = |key: &str| ComposeProject {
            key: key.into(),
            ..Default::default()
        };
        // Act / Assert
        assert!(
            compose_input(run(), vec![project("extra")])
                .unwrap_err()
                .to_string()
                .contains("Compose user apps")
        );
        let mut input = run();
        input.system_apps = vec!["missing".into()];
        assert!(compose_input(input, vec![]).is_err());
        let mut input = run();
        input.system_apps = vec!["monitor".into()];
        assert!(compose_input(input, vec![project("monitor"), project("monitor")]).is_err());
        let mut input = run();
        input.dora_apps = vec!["monitor".into()];
        input.system_apps = vec!["monitor".into()];
        assert!(compose_input(input, vec![project("monitor")]).is_err());
        let mut input = run();
        input.stop_seconds = 0;
        assert!(compose_input(input, vec![]).is_err());
    }
}
