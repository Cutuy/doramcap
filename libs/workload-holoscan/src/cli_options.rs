//! Literal encoding of Console-owned Docker options at the native CLI string boundary.
use compose_apps::compose::adapter::repository_scope;
use docker_engine::{ARTIFACTS_LABEL, CONTROLLER_LABEL, RUN_LABEL};
use std::path::Path;
use workload_lifecycle::launch::RunPlan;

/// Encode only Console ownership, cidfile and existing app placement; native options remain native.
/// Requires admitted canonical repository/absolute artifact and cidfile paths, plus nonempty placement.
/// CLI5 expands `$` even inside quotes, so values containing it or NUL are rejected before launch.
pub fn owned(run: &RunPlan, cgroup_parent: &str, cidfile: &Path) -> Result<String, String> {
    validate(run, cidfile)?;
    let scope = repository_scope(Path::new(&run.repository_root));
    let scope = scope
        .to_str()
        .ok_or("Holoscan controller scope must be UTF-8")?;
    let cidfile = cidfile.to_str().ok_or("Holoscan cidfile must be UTF-8")?;
    let tokens = [
        format!("--label={CONTROLLER_LABEL}={scope}"),
        format!("--label={RUN_LABEL}={}", run.id),
        format!("--label={ARTIFACTS_LABEL}={}", run.artifacts),
        format!("--cidfile={cidfile}"),
        format!("--cgroup-parent={cgroup_parent}"),
    ];
    if tokens.iter().any(|value| value.contains('$')) {
        return Err(
            "Holoscan CLI5 expands '$' in Docker options; Console-owned values must contain no '$'"
                .into(),
        );
    }
    shlex::try_join(tokens.iter().map(String::as_str))
        .map_err(|_| "Holoscan Console-owned Docker options must contain no NUL".into())
}

/// Validate user-derived owned values before reset, placement or process side effects.
/// The generated cgroup parent is checked by `owned` once the helper resolves it.
pub fn validate(run: &RunPlan, cidfile: &Path) -> Result<(), String> {
    if !Path::new(&run.repository_root).is_absolute()
        || !Path::new(&run.artifacts).is_absolute()
        || !cidfile.is_absolute()
        || run.id.is_empty()
    {
        return Err(
            "Holoscan owned Docker options require absolute paths and nonempty identity".into(),
        );
    }
    for value in [
        run.id.as_str(),
        run.artifacts.as_str(),
        cidfile.to_str().ok_or("Holoscan cidfile must be UTF-8")?,
    ] {
        literal(value)?;
    }
    Ok(())
}
fn literal(value: &str) -> Result<(), String> {
    if value.contains('$') {
        return Err(
            "Holoscan CLI5 expands '$' in Docker options; Console-owned values must contain no '$'"
                .into(),
        );
    }
    if value.contains('\0') {
        return Err("Holoscan Console-owned Docker options must contain no NUL".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_preserve_literal_boundaries() {
        // Arrange
        let run = RunPlan {
            id: "build 'quoted'\nnext".into(),
            repository_root: "/tmp/project".into(),
            artifacts: "/tmp/data 'quoted'\nnext".into(),
            ..Default::default()
        };
        // Act
        let encoded = owned(&run, "app.slice", Path::new("/tmp/data 'quoted'\nnext/cid")).unwrap();
        let tokens = shlex::split(&encoded).unwrap();
        // Assert
        assert_eq!(tokens.len(), 5);
        assert_eq!(
            tokens[0],
            format!(
                "--label={CONTROLLER_LABEL}={}",
                repository_scope(Path::new(&run.repository_root)).display()
            )
        );
        assert_eq!(tokens[1], "--label=io.doramcap.run=build 'quoted'\nnext");
        assert_eq!(
            tokens[2],
            "--label=io.doramcap.artifacts=/tmp/data 'quoted'\nnext"
        );
        assert_eq!(tokens[3], "--cidfile=/tmp/data 'quoted'\nnext/cid");
    }

    #[test]
    fn native_expansion_and_nul_are_explicit_errors() {
        // Arrange
        let mut run = RunPlan {
            id: "build".into(),
            repository_root: "/tmp/project".into(),
            artifacts: "/tmp/data".into(),
            ..Default::default()
        };
        // Act / Assert
        run.artifacts = "/tmp/$HOME".into();
        assert!(
            owned(&run, "app.slice", Path::new("/tmp/cid"))
                .unwrap_err()
                .contains("CLI5 expands")
        );
        run.artifacts = "/tmp/data\0".into();
        assert!(
            owned(&run, "app.slice", Path::new("/tmp/cid"))
                .unwrap_err()
                .contains("NUL")
        );
        run.artifacts = "/tmp/data".into();
        assert!(owned(&run, "$slice", Path::new("/tmp/cid")).is_err());
        assert!(owned(&run, "app.slice", Path::new("/tmp/$cid")).is_err());
    }
}
