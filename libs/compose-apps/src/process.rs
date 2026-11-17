//! Docker placement of the shared admitted foreground-helper owner.
use crate::cgroup::{DockerCgroup, Role, placement_for_run, service_parent};
use docker_engine::Engine;
use std::{path::Path, time::Duration};
pub use workload_lifecycle::process::{OWNERSHIP_FILE, repository_scope};
use workload_lifecycle::{launch::RunPlan, process::HelperProcess};

/// Prepare one shared helper under the existing application hierarchy, before any mutation/copy.
pub async fn prepare(
    engine: &Engine,
    run: &RunPlan,
    role: Role,
    component: &str,
    admission: &workload_lifecycle::admission::Admission,
) -> Result<HelperProcess, String> {
    if component.is_empty()
        || !component
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(
            "helper component must be a single alphanumeric, dash or underscore component".into(),
        );
    }
    let placement = placement_for_run(engine, run)
        .await
        .map_err(|e| e.to_string())?;
    let parent = service_parent(&placement, role, component)?;
    HelperProcess::prepare_at(
        placement,
        Path::new(&run.artifacts),
        Path::new(&run.repository_root),
        parent,
        Duration::from_secs(run.stop_seconds),
    )?
    .with_admission(admission.clone())
}
/// Remove only this backend's empty parents, then acknowledge the shared current record.
pub async fn remove(helper: &HelperProcess, stop: Duration) -> Result<(), String> {
    validate(helper)?;
    helper.remove_leaf().await?;
    DockerCgroup::group(helper.placement().clone(), stop)
        .remove()
        .await?;
    helper.clear()
}

/// Refuse unknown/cross-form ownership before signals; the original backend must complete cleanup.
pub fn validate(helper: &HelperProcess) -> Result<(), String> {
    let placement = helper.placement();
    let matched = [Role::System, Role::User].into_iter().any(|role| {
        service_parent(placement, role, "")
            .ok()
            .is_some_and(|prefix| {
                let prefix = prefix.strip_suffix(".slice").unwrap_or(&prefix);
                let parent = helper.cgroup_parent();
                let parent = parent.strip_suffix(".slice").unwrap_or(parent);
                parent
                    .strip_prefix(prefix)
                    .is_some_and(|app| !app.is_empty() && !app.contains('/') && !app.contains('.'))
            })
    });
    if matched {
        Ok(())
    } else {
        Err("helper cleanup incomplete: use the original systemd workflow to remove its remaining resources".into())
    }
}
