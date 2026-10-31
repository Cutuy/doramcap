//! Request validation before native input normalization.
use super::*;
use crate::compose::adapter::validate_name;
use crate::workflows::dora;
use std::collections::HashSet;

/// Clone app inputs under the experiment identity and approved runtime directory.
pub(super) fn app_input(
    app: &ComposeProject,
    runtime: &dora::DoraRuntime,
) -> Result<ComposeProject> {
    let mut input = app.clone();
    input.key = "experiment".into();
    if input
        .environment
        .insert(
            "DORAMCAP_DORA_RUNTIME_DIR".into(),
            runtime.runtime_directory.clone(),
        )
        .is_some_and(|old| old != runtime.runtime_directory)
    {
        return Err(invalid("conflicting Dora interpolation setting"));
    }
    Ok(input)
}

/// Validate run identity, paths, app keys and budgets before creating artifacts.
pub(super) fn validate(spec: &ComposeRun) -> Result<()> {
    validate_name(&spec.id)?;
    if !spec.repository_root.is_absolute() {
        return Err(invalid("repository root must be absolute"));
    }
    super::super::validate_budgets(spec.readiness_seconds, spec.stop_seconds)?;
    if (spec.projects.is_empty() && spec.system_apps.is_empty())
        || std::iter::once(&spec.artifacts).any(|path| !path.is_absolute())
    {
        return Err(invalid("require absolute artifact paths and app inputs"));
    }
    let mut keys = HashSet::new();
    for p in spec.system_apps.iter().chain(&spec.projects) {
        validate_name(&p.key)?;
        if !Path::new(&p.directory).is_absolute() || !keys.insert(&p.key) {
            return Err(invalid("app directories must be absolute and keys unique"));
        }
    }
    Ok(())
}
