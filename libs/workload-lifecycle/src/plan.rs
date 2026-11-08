//! Validation of run settings and app identities shared by all launch forms.
use crate::launch::RunPlan;
use std::collections::BTreeSet;

/// Host filesystem containing explicitly shared live entries.
pub const HOST_SHM_ROOT: &str = "/dev/shm";

/// Reject host shared-memory names that are not portable single path components.
pub fn validate_host_shm(names: &[String]) -> Result<(), String> {
    if names.iter().any(|name| {
        name.is_empty()
            || name == "."
            || name == ".."
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    }) {
        return Err("host_shm entries must use ASCII letters, digits, dot, underscore or hyphen; empty, . and .. are invalid".into());
    }
    Ok(())
}

/// Validate caller inputs and reject empty, duplicate or unknown app IDs and roles.
pub fn validate_apps<'a>(
    run: &RunPlan,
    apps: impl IntoIterator<Item = &'a str>,
) -> Result<(), String> {
    validate_host_shm(&run.host_shm)?;
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
    fn host_shared_memory_names_are_plain_basenames() {
        // Arrange
        // Act / Assert
        for name in ["camera", "Camera_1.v2", "camera-0", ".driver"] {
            validate_host_shm(&[name.into()]).unwrap();
        }
        for name in [
            "",
            ".",
            "..",
            "driver/frame",
            "camera space",
            "camera:0",
            "camera\\0",
            "camera%0",
            "camera\"0",
            "camera\n0",
            "caméra",
        ] {
            assert!(
                validate_host_shm(&[name.into()])
                    .unwrap_err()
                    .contains("host_shm"),
                "{name:?}"
            );
        }
    }

    #[test]
    fn app_roles_must_reference_unique_launch_inputs() {
        // Arrange
        let run = RunPlan {
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
}
