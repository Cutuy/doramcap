//! Native Run argv; only Console-owned Docker tokens pass through the shared encoder.
use super::views;
use crate::{HoloscanPlan, cli_options};
use host_filesystem::overlay::OverlaySet;
use std::path::Path;
use workload_lifecycle::launch::RunPlan;

/// Preserve native selection/defaults and use the prepared controlled views without image substitution.
/// Other native options remain with the CLI; callers configure the approved container environment.
pub(super) fn run(
    run: &RunPlan,
    plan: &HoloscanPlan,
    views: &OverlaySet,
    cgroup_parent: &str,
    cidfile: &Path,
) -> Result<Vec<String>, String> {
    let selection = plan
        .selection
        .as_ref()
        .ok_or("Holoscan selection is required")?;
    let mappings = views::mappings(views, run, plan)?;
    let project = mappings[0];
    if project.original != Path::new(&run.repository_root)
        || project.effective.file_name() != project.original.file_name()
    {
        return Err("project view must preserve the canonical Build root and its basename".into());
    }
    let mut arguments = vec![
        "--project-root".into(),
        utf8(&project.effective)?.into(),
        "run".into(),
    ];
    if !selection.language.is_empty() {
        arguments.extend([
            "--language".into(),
            crate::discovery::normalize_language(&selection.language),
        ]);
    }
    arguments.extend([
        "--no-docker-build".into(),
        "--no-local-build".into(),
        "--persistent".into(),
    ]);
    for (mapping, supplied) in mappings[1..].iter().zip(&plan.extra_data_folders) {
        let folder = if mapping.effective == mapping.original {
            // Native abspath preserves the supplied basename for direct symlink aliases.
            supplied.as_str()
        } else {
            if mapping.effective.file_name() != Path::new(supplied).file_name() {
                return Err("extra data view does not preserve its native volume basename".into());
            }
            utf8(&mapping.effective)?
        };
        arguments.extend(["--add-volume".into(), folder.into()]);
    }
    arguments.push(format!(
        "--docker-opts={}",
        cli_options::owned(run, cgroup_parent, cidfile)?
    ));
    arguments.extend(["--".into(), selection.application.clone()]);
    if !selection.mode.is_empty() {
        arguments.push(selection.mode.clone());
    }
    Ok(arguments)
}
fn utf8(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| "native launch paths must be UTF-8".into())
}
