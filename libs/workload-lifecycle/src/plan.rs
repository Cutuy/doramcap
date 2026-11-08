//! Validation of app identities shared by all launch forms.
use crate::launch::RunPlan;
use std::collections::BTreeSet;

/// Reject empty, duplicate or unknown app IDs and roles before launching resources.
pub fn validate_apps<'a>(
    run: &RunPlan,
    apps: impl IntoIterator<Item = &'a str>,
) -> Result<(), String> {
    let mut known = BTreeSet::new();
    for app in apps {
        if app.is_empty() || !known.insert(app) {
            return Err(format!("empty or duplicate app ID: {app:?}"));
        }
    }
    for (role, apps) in [
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
