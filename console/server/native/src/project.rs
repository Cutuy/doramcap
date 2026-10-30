//! Conservative discovery of a Compose project beneath the configured root.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use workload_docker::compose::ComposeProject;

/// Discover a single default Compose entry point and its optional override and env file.
pub fn discover(
    root: &Path,
    directory: &Path,
    profiles: Vec<String>,
) -> Result<ComposeProject, String> {
    if !directory.is_absolute() {
        return Err("project directory must be absolute".into());
    }
    let root = root
        .canonicalize()
        .map_err(|e| format!("project root: {e}"))?;
    let directory = directory
        .canonicalize()
        .map_err(|e| format!("project directory: {e}"))?;
    if !directory.is_dir() || !directory.starts_with(&root) {
        return Err("project directory is outside the approved root or is not a directory".into());
    }
    let bases = [
        "compose.yaml",
        "compose.yml",
        "docker-compose.yaml",
        "docker-compose.yml",
    ];
    let mut found = Vec::new();
    for name in bases {
        if present_file(&directory.join(name))? {
            found.push(name);
        }
    }
    if found.len() != 1 {
        return Err("project must contain exactly one default Compose file".into());
    }
    let base = found[0];
    let prefix = if base.starts_with("docker-compose.") {
        "docker-compose"
    } else {
        "compose"
    };
    let mut files = vec![PathBuf::from(base)];
    for suffix in ["yaml", "yml"] {
        let name = format!("{prefix}.override.{suffix}");
        if present_file(&directory.join(&name))? {
            files.push(name.into());
        }
    }
    if files.len() > 2 {
        return Err("ambiguous Compose override files".into());
    }
    let env_files = if present_file(&directory.join(".env"))? {
        vec![PathBuf::from(".env")]
    } else {
        vec![]
    };
    Ok(ComposeProject {
        key: "workload".into(),
        directory,
        files,
        env_files,
        environment: BTreeMap::new(),
        profiles,
        services: vec![],
        completed_services: vec![],
        remove_volumes: false,
    })
}

fn present_file(path: &Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            if !path.is_file() {
                return Err(format!("{} is not a regular file", path.display()));
            }
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}
