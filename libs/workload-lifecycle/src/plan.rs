//! Validation of app identities shared by all launch forms.
use crate::launch::{HostShmAccess, RunPlan};
use std::collections::BTreeSet;

/// Require explicit host shm access before preparation creates any resources.
pub fn validate_host_shm_access(value: i32) -> Result<HostShmAccess, String> {
    match HostShmAccess::try_from(value) {
        Ok(access @ (HostShmAccess::Passthrough | HostShmAccess::Blocked)) => Ok(access),
        _ => Err(
            "host_shm_access must be HOST_SHM_ACCESS_PASSTHROUGH or HOST_SHM_ACCESS_BLOCKED".into(),
        ),
    }
}

/// Validate caller inputs and reject empty, duplicate or unknown app IDs and roles.
pub fn validate_apps<'a>(
    run: &RunPlan,
    apps: impl IntoIterator<Item = &'a str>,
) -> Result<(), String> {
    validate_host_shm_access(run.host_shm_access)?;
    host_filesystem::overlay::Disposition::from_policy(run.write_policy)
        .map_err(|error| error.to_string())?;
    if run.dora_runtime.is_some() {
        return Err("Dora runtime is filled by preparation; caller inputs must omit it".into());
    }
    let mut known = BTreeSet::new();
    for app in apps {
        if app.is_empty() || !known.insert(app) {
            return Err(format!("empty or duplicate app ID: {app:?}"));
        }
    }
    for (role, apps) in [
        ("Dora", &run.dora_apps),
        ("system", &run.system_apps),
        ("MCAP writer", &run.mcap_apps),
    ] {
        let mut seen = BTreeSet::new();
        for app in apps {
            if !known.contains(app.as_str()) || !seen.insert(app) {
                return Err(format!("unknown or duplicate {role} app: {app:?}"));
            }
        }
    }
    if run
        .dora_apps
        .iter()
        .any(|id| run.system_apps.contains(id) || run.mcap_apps.contains(id))
    {
        return Err("Dora apps cannot also be system apps or recording writers".into());
    }
    if !run.dataflow_file.is_empty()
        && (run.dora_apps.is_empty() || !std::path::Path::new(&run.dataflow_file).is_absolute())
    {
        return Err("dataflow_file needs Dora apps and an absolute path".into());
    }
    Ok(())
}

/// Fixed startup phases; shutdown traverses them in reverse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Dora,
    System,
    User,
}

/// Classify a validated app by its shared role.
pub fn phase(run: &RunPlan, id: &str) -> Phase {
    if run.dora_apps.iter().any(|app| app == id) {
        Phase::Dora
    } else if run.system_apps.iter().any(|app| app == id) {
        Phase::System
    } else {
        Phase::User
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_roles_must_reference_unique_launch_inputs() {
        // Arrange
        let run = RunPlan {
            host_shm_access: HostShmAccess::Blocked as i32,
            system_apps: vec!["monitor".into()],
            mcap_apps: vec!["monitor".into()],
            ..Default::default()
        };
        // Act / Assert
        assert!(validate_apps(&run, ["monitor", "user"]).is_ok());
        assert!(validate_apps(&run, ["user"]).is_err());
        assert!(validate_apps(&run, ["monitor", "monitor"]).is_err());
        assert!(validate_apps(&run, ["monitor", ""]).is_err());
        // Act / Assert: every backend prepares fresh runtime values from caller inputs.
        let mut prepared = run.clone();
        prepared.dora_runtime = Some(Default::default());
        assert!(
            validate_apps(&prepared, ["monitor", "user"])
                .unwrap_err()
                .contains("caller inputs must omit")
        );
        for writer in [false, true] {
            // Arrange
            let mut duplicate = run.clone();
            if writer {
                duplicate.mcap_apps.push("monitor".into());
            } else {
                duplicate.system_apps.push("monitor".into());
            }
            // Act / Assert
            assert!(validate_apps(&duplicate, ["monitor", "user"]).is_err());
        }
    }

    #[test]
    fn existing_run_json_defaults_to_no_caller_private_records() {
        // Arrange
        let run = RunPlan::default();
        let mut saved = serde_json::to_value(&run).unwrap();
        saved.as_object_mut().unwrap().remove("private_records");
        // Act
        let decoded: RunPlan = serde_json::from_value(saved).unwrap();
        // Assert
        assert!(decoded.private_records.is_empty());
    }

    #[test]
    fn runs_require_explicit_host_shm_access() {
        // Arrange
        let mut run = RunPlan::default();
        for value in [0, -1, 3, i32::MAX] {
            // Arrange
            run.host_shm_access = value;
            // Act
            let error = validate_apps(&run, ["app"]).unwrap_err();
            // Assert
            assert_eq!(
                error,
                "host_shm_access must be HOST_SHM_ACCESS_PASSTHROUGH or HOST_SHM_ACCESS_BLOCKED"
            );
        }
        for access in [HostShmAccess::Passthrough, HostShmAccess::Blocked] {
            // Arrange
            run.host_shm_access = access as i32;
            // Act
            let result = validate_apps(&run, ["app"]);
            // Assert
            assert!(result.is_ok());
        }
    }
}
