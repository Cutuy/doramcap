//! Raw native project inputs shared by later preparation and build operations.
use crate::{DeclaredProjectKind, DockerOptionTokens, NativeInputFile, ProjectDockerInputs};
use orchestrator::{Error, Result};
use serde_json::Value;
use std::path::Path;

const METADATA_FILE: &str = "metadata.json";
const CONFIGURATION_FILE: &str = "pyproject.toml";
const DECLARATIONS: &[&str] = &[
    "application",
    "benchmark",
    "gxf_extension",
    "module",
    "operator",
    "package",
    "tutorial",
];

/// Read raw root declarations and optional Module Docker token arrays.
/// Returned values preserve whitespace and carry identities of the files read.
/// The CLI owns validation and whether a declared Module context takes effect.
///
/// # Inputs and files
/// `root` is an existing absolute UTF-8 directory. Missing metadata declares a source root.
/// Application/source declarations skip TOML; a missing/non-file pyproject.toml supplies no defaults.
/// Reads materialize each complete file, so retained memory grows with input size.
///
/// # Observations
/// Multiple recognized envelopes are Ambiguous. Neither envelopes nor catalog entries prove native context.
/// Digests identify whole file bytes, including unrelated TOML; they are not Docker-only fingerprints.
///
/// # Errors
/// Malformed JSON/TOML, non-object declaration values and invalid Docker arrays report the file and constraint.
/// Errors omit source contents. Other Module identity, profile and Dockerfile validation stays with the CLI.
pub async fn read_project_inputs(root: &Path) -> Result<ProjectDockerInputs> {
    let project_root = root
        .to_str()
        .filter(|_| root.is_absolute())
        .ok_or_else(|| {
            Error::Configuration("Holoscan project root must be an absolute UTF-8 path".into())
        })?;
    if !tokio::fs::metadata(root)
        .await
        .is_ok_and(|metadata| metadata.is_dir())
    {
        return Err(invalid(root, "project root must be an existing directory"));
    }
    let mut inputs = ProjectDockerInputs {
        project_root: project_root.into(),
        declared_kind: DeclaredProjectKind::Source as i32,
        ..Default::default()
    };
    let metadata_path = root.join(METADATA_FILE);
    let Some(bytes) = read_optional(&metadata_path, false).await? else {
        return Ok(inputs);
    };
    inputs.metadata = Some(identity(&metadata_path, &bytes));
    let metadata: Value = serde_json::from_slice(&bytes).map_err(|error| {
        invalid(
            &metadata_path,
            &format!(
                "invalid JSON ({:?}, line {}, column {})",
                error.classify(),
                error.line(),
                error.column()
            ),
        )
    })?;
    let object = metadata
        .as_object()
        .ok_or_else(|| invalid(&metadata_path, "root declaration must be a JSON object"))?;
    for &name in DECLARATIONS {
        if let Some(value) = object.get(name) {
            if !value.is_object() {
                return Err(invalid(
                    &metadata_path,
                    &format!("{name} declaration must be an object"),
                ));
            }
            inputs.declarations.push(name.into());
        }
    }
    inputs.declared_kind = if inputs.declarations.len() > 1 {
        DeclaredProjectKind::Ambiguous
    } else if object.contains_key("module") {
        DeclaredProjectKind::Module
    } else if object.contains_key("application") {
        DeclaredProjectKind::Application
    } else {
        DeclaredProjectKind::Source
    } as i32;
    // Only a raw Module declaration makes the optional Module file relevant to this observation.
    if !object.contains_key("module") {
        return Ok(inputs);
    }
    let configuration_path = root.join(CONFIGURATION_FILE);
    let Some(bytes) = read_optional(&configuration_path, true).await? else {
        return Ok(inputs);
    };
    inputs.configuration = Some(identity(&configuration_path, &bytes));
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| invalid(&configuration_path, "TOML input must be UTF-8"))?;
    let configuration: toml::Value = toml::from_str(text).map_err(|error: toml::de::Error| {
        let location = error
            .span()
            .map(|span| format!(" at bytes {}..{}", span.start, span.end))
            .unwrap_or_default();
        invalid(&configuration_path, &format!("invalid TOML{location}"))
    })?;
    let Some(tool) = configuration.get("tool") else {
        return Ok(inputs);
    };
    let tool = tool
        .as_table()
        .ok_or_else(|| invalid(&configuration_path, "tool must be a table"))?;
    let Some(holoscan) = tool.get("holoscan") else {
        return Ok(inputs);
    };
    let holoscan = holoscan
        .as_table()
        .ok_or_else(|| invalid(&configuration_path, "tool.holoscan must be a table"))?;
    inputs.docker_build_args = tokens(
        holoscan.get("docker-build-args"),
        &configuration_path,
        "docker-build-args",
    )?;
    inputs.docker_run_args = tokens(
        holoscan.get("docker-run-args"),
        &configuration_path,
        "docker-run-args",
    )?;
    Ok(inputs)
}

async fn read_optional(path: &Path, non_file_is_absent: bool) -> Result<Option<Vec<u8>>> {
    match tokio::fs::metadata(path).await {
        Ok(metadata) if !metadata.is_file() => {
            return if non_file_is_absent {
                Ok(None)
            } else {
                Err(invalid(path, "input must be a regular file"))
            };
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Error::message(format!(
                "cannot inspect {}: {error}",
                path.display()
            )));
        }
        _ => {}
    }
    tokio::fs::read(path)
        .await
        .map(Some)
        .map_err(|error| Error::message(format!("cannot read {}: {error}", path.display())))
}
fn identity(path: &Path, bytes: &[u8]) -> NativeInputFile {
    NativeInputFile {
        path: path.to_str().expect("UTF-8 project path").into(),
        sha256: host_filesystem::files::digest(bytes),
    }
}
fn invalid(path: &Path, constraint: &str) -> Error {
    Error::Configuration(format!("{}: {constraint}", path.display()))
}
fn tokens(
    value: Option<&toml::Value>,
    path: &Path,
    field: &str,
) -> Result<Option<DockerOptionTokens>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let array = value.as_array().ok_or_else(|| {
        invalid(
            path,
            &format!("tool.holoscan.{field} must be an array of nonempty strings"),
        )
    })?;
    let mut values = Vec::with_capacity(array.len());
    for (index, value) in array.iter().enumerate() {
        let value = value
            .as_str()
            .filter(|value| value.chars().any(|character| !python_whitespace(character)))
            .ok_or_else(|| {
                invalid(
                    path,
                    &format!("tool.holoscan.{field}[{index}] must be a nonempty string"),
                )
            })?;
        values.push(value.into());
    }
    Ok(Some(DockerOptionTokens { values }))
}
fn python_whitespace(character: char) -> bool {
    character.is_whitespace() || matches!(character, '\u{1c}'..='\u{1f}')
}
