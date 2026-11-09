//! Compare published Compose models against the installed Compose CLI.
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{collections::BTreeSet, error::Error, fs, path::Path, process::Command};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Serialize)]
struct Observation {
    fixture: String,
    crate_name: &'static str,
    parse_error: Option<String>,
    serialization_changes: Vec<String>,
    compose_error: Option<String>,
    normalized_changes: Option<Vec<String>>,
}

#[derive(Serialize)]
struct Report {
    compose_version: String,
    observations: Vec<Observation>,
}

// Report every differing JSON pointer; values stay out of the report.
fn differences(before: &Value, after: &Value, path: &str, output: &mut Vec<String>) {
    match (before, after) {
        (Value::Object(a), Value::Object(b)) => {
            for key in a.keys().chain(b.keys()).collect::<BTreeSet<_>>() {
                let path = format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"));
                match (a.get(key), b.get(key)) {
                    (Some(a), Some(b)) => differences(a, b, &path, output),
                    _ => output.push(path),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                differences(a, b, &format!("{path}/{i}"), output);
            }
        }
        _ if before != after => output.push(path.into()),
        _ => {}
    }
}

fn normalize(file: &Path, directory: &Path) -> Result<Value> {
    let output = Command::new("docker")
        .args([
            "compose",
            "--project-name",
            "type-evaluation",
            "--project-directory",
        ])
        .arg(directory)
        .arg("-f")
        .arg(file)
        .args(["config", "--format", "json"])
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_owned()
            .into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn evaluate<T: DeserializeOwned + Serialize>(
    fixture: &str,
    crate_name: &'static str,
    model: &Value,
    directory: &Path,
) -> Result<Observation> {
    let mut observation = Observation {
        fixture: fixture.into(),
        crate_name,
        parse_error: None,
        serialization_changes: vec![],
        compose_error: None,
        normalized_changes: None,
    };
    let typed = match serde_json::from_value::<T>(model.clone()) {
        Ok(typed) => typed,
        Err(error) => {
            observation.parse_error = Some(error.to_string());
            return Ok(observation);
        }
    };
    let output = serde_json::to_value(typed)?;
    differences(model, &output, "", &mut observation.serialization_changes);
    let temporary = tempfile::tempdir()?;
    let file = temporary.path().join("compose.json");
    fs::write(&file, serde_json::to_vec(&output)?)?;
    match normalize(&file, directory) {
        Ok(rendered) => {
            let mut changes = Vec::new();
            differences(model, &rendered, "", &mut changes);
            observation.normalized_changes = Some(changes);
        }
        Err(error) => observation.compose_error = Some(error.to_string()),
    }
    Ok(observation)
}

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut fixtures: Vec<_> = fs::read_dir(root.join("fixtures"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<_>>()?;
    fixtures.retain(|path| path.extension().is_some_and(|ext| ext == "yaml"));
    fixtures.sort();
    for role in ["dora-coordinator", "dora-daemon"] {
        fixtures.push(root.join(format!("../../deploy/{role}/compose.yaml")));
    }
    let version = Command::new("docker")
        .args(["compose", "version", "--short"])
        .output()?;
    if !version.status.success() {
        return Err("docker compose version failed".into());
    }
    let mut report = Report {
        compose_version: String::from_utf8(version.stdout)?.trim().into(),
        observations: vec![],
    };
    for file in fixtures {
        let file = fs::canonicalize(file)?;
        let directory = file.parent().ok_or("fixture has no parent")?;
        let name = if file.file_name().is_some_and(|name| name == "compose.yaml") {
            directory
                .file_name()
                .ok_or("fixture has no name")?
                .to_string_lossy()
                .into_owned()
        } else {
            file.file_stem()
                .ok_or("fixture has no name")?
                .to_string_lossy()
                .into_owned()
        };
        let model = normalize(&file, directory).map_err(|e| format!("{name}: {e}"))?;
        // Establish a raw JSON control before attributing drift to a crate.
        let control = evaluate::<Value>(&name, "serde_json::Value (control)", &model, directory)?;
        if control
            .normalized_changes
            .as_ref()
            .is_none_or(|changes| !changes.is_empty())
        {
            return Err(format!(
                "{name}: raw JSON control changed: {}",
                serde_json::to_string(&control)?
            )
            .into());
        }
        report
            .observations
            .push(evaluate::<docker_compose_types::Compose>(
                &name,
                "docker-compose-types 0.24.0",
                &model,
                directory,
            )?);
        report.observations.push(evaluate::<compose_spec::Compose>(
            &name,
            "compose_spec 0.3.0",
            &model,
            directory,
        )?);
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
